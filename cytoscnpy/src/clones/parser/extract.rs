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
    let module = AstParser::parse(source)?;
    let mut subtrees = Vec::new();
    extract_from_body(&module.body, path, source, &mut subtrees, false);
    Ok(subtrees)
}

/// Recursively extract subtrees from a statement body
fn extract_from_body(
    body: &[Stmt],
    path: &PathBuf,
    source: &str,
    subtrees: &mut Vec<Subtree>,
    in_class: bool,
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

                if (end_line - start_line + 1) >= crate::constants::MIN_CLONE_LINES {
                    subtrees.push(Subtree {
                        node_type,
                        name: Some(f.name.to_string()),
                        start_byte,
                        end_byte,
                        start_line,
                        end_line,
                        file: path.clone(),
                        source_slice: source[start_byte..end_byte].to_string(),
                        children: extract_stmt_nodes(&f.body),
                    });
                }

                extract_from_body(&f.body, path, source, subtrees, false);
            }
            Stmt::ClassDef(c) => {
                let start_byte = c.range().start().to_usize();
                let end_byte = c.range().end().to_usize();
                let (start_line, end_line) = byte_to_lines(start_byte, end_byte, source);

                if (end_line - start_line + 1) >= crate::constants::MIN_CLONE_LINES {
                    subtrees.push(Subtree {
                        node_type: SubtreeType::Class,
                        name: Some(c.name.to_string()),
                        start_byte,
                        end_byte,
                        start_line,
                        end_line,
                        file: path.clone(),
                        source_slice: source[start_byte..end_byte].to_string(),
                        children: extract_stmt_nodes(&c.body),
                    });
                }

                extract_from_body(&c.body, path, source, subtrees, true);
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
