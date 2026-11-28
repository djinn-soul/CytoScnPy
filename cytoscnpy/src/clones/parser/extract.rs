use super::definitions::{class_nodes, function_nodes};
use super::enum_members::preserve_semantics;
use super::expressions::demote_sparse_subscript_keys;
use super::interfaces::{is_pure_interface_class, is_simple_exception_class};
use super::qualnames::ImportAliases;
use super::schema_fields::preserve_field_semantics;
use super::statements::extract_stmt_nodes;
use super::types::{AstParser, Subtree, SubtreeType};
use crate::clones::CloneError;
use ruff_python_ast::Stmt;
use ruff_text_size::Ranged;
use std::path::PathBuf;

/// Extract function and class subtrees from source code
///
/// # Errors
/// Returns error if parsing fails
pub fn extract_subtrees(source: &str, path: &PathBuf) -> Result<Vec<Subtree>, CloneError> {
    extract_subtrees_with_min_lines(source, path, crate::constants::MIN_CLONE_LINES)
}

pub(crate) fn extract_subtrees_with_min_lines(
    source: &str,
    path: &PathBuf,
    min_lines: usize,
) -> Result<Vec<Subtree>, CloneError> {
    let module = AstParser::parse(source)?;
    Ok(extract_subtrees_ast(source, path, &module.body, min_lines))
}

pub(crate) fn extract_subtrees_ast(
    source: &str,
    path: &PathBuf,
    body: &[Stmt],
    min_lines: usize,
) -> Vec<Subtree> {
    let mut subtrees = Vec::new();
    let ctx = ExtractContext {
        path,
        source,
        min_lines,
        imports: ImportAliases::from_module(body),
    };
    extract_from_body(&ctx, body, &mut subtrees, false);
    subtrees
}

/// Per-file state shared by the recursive walk.
struct ExtractContext<'a> {
    path: &'a PathBuf,
    source: &'a str,
    min_lines: usize,
    imports: ImportAliases,
}

/// Recursively extract subtrees from a statement body
fn extract_from_body(
    ctx: &ExtractContext<'_>,
    body: &[Stmt],
    subtrees: &mut Vec<Subtree>,
    in_class: bool,
) {
    let ExtractContext {
        path,
        source,
        min_lines,
        ..
    } = *ctx;
    for stmt in body {
        match stmt {
            Stmt::FunctionDef(f) => {
                let start_byte = f.range().start().to_usize();
                let end_byte = f.range().end().to_usize();
                let (start_line, end_line) = byte_to_lines(start_byte, end_byte, source);

                let node_type = if in_class {
                    SubtreeType::Method
                } else if f.is_async {
                    SubtreeType::AsyncFunction
                } else {
                    SubtreeType::Function
                };

                if end_line - start_line + 1 >= min_lines {
                    let mut children = function_nodes(f);
                    children.extend(extract_stmt_nodes(&f.body));
                    demote_sparse_subscript_keys(&mut children);
                    subtrees.push(Subtree {
                        node_type,
                        name: Some(f.name.to_string()),
                        start_byte,
                        end_byte,
                        start_line,
                        end_line,
                        file: path.clone(),
                        source_slice: source[start_byte..end_byte].to_string(),
                        children,
                    });
                }

                extract_from_body(ctx, &f.body, subtrees, false);
            }
            Stmt::ClassDef(c) => {
                let start_byte = c.range().start().to_usize();
                let end_byte = c.range().end().to_usize();
                let (start_line, end_line) = byte_to_lines(start_byte, end_byte, source);

                // Check exclusions before building the (costly) body subtree.
                if end_line - start_line + 1 >= min_lines
                    && !is_pure_interface_class(c)
                    && !is_simple_exception_class(c)
                {
                    let mut children = class_nodes(c);
                    let mut body_nodes = extract_stmt_nodes(&c.body);
                    preserve_semantics(c, &ctx.imports, &mut body_nodes);
                    preserve_field_semantics(c, &ctx.imports, &mut body_nodes);
                    children.extend(body_nodes);
                    demote_sparse_subscript_keys(&mut children);
                    subtrees.push(Subtree {
                        node_type: SubtreeType::Class,
                        name: Some(c.name.to_string()),
                        start_byte,
                        end_byte,
                        start_line,
                        end_line,
                        file: path.clone(),
                        source_slice: source[start_byte..end_byte].to_string(),
                        children,
                    });
                }

                extract_from_body(ctx, &c.body, subtrees, true);
            }
            _ => {}
        }
    }
}

/// Convert byte offsets to line numbers
fn byte_to_lines(start_byte: usize, end_byte: usize, source: &str) -> (usize, usize) {
    let start_line = source[..start_byte].matches('\n').count() + 1;
    let end_line = source[..end_byte].matches('\n').count() + 1;
    (start_line, end_line)
}
