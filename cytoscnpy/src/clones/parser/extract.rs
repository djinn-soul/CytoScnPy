use super::definitions::{class_nodes, function_nodes};
use super::enum_members::preserve_semantics;
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
    let mut subtrees = Vec::new();
    extract_from_body(&module.body, path, source, &mut subtrees, false, min_lines);
    Ok(subtrees)
}

/// Recursively extract subtrees from a statement body
fn extract_from_body(
    body: &[Stmt],
    path: &PathBuf,
    source: &str,
    subtrees: &mut Vec<Subtree>,
    in_class: bool,
    min_lines: usize,
) {
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

                let mut children = function_nodes(f);
                children.extend(extract_stmt_nodes(&f.body));
                if end_line - start_line + 1 >= min_lines {
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

                extract_from_body(&f.body, path, source, subtrees, false, min_lines);
            }
            Stmt::ClassDef(c) => {
                let start_byte = c.range().start().to_usize();
                let end_byte = c.range().end().to_usize();
                let (start_line, end_line) = byte_to_lines(start_byte, end_byte, source);

                let mut children = class_nodes(c);
                let mut body_nodes = extract_stmt_nodes(&c.body);
                preserve_semantics(c, &mut body_nodes);
                children.extend(body_nodes);
                if end_line - start_line + 1 >= min_lines {
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

                extract_from_body(&c.body, path, source, subtrees, true, min_lines);
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
