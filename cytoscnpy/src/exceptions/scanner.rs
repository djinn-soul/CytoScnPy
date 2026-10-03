//! Python AST scanner for bare-except and empty exception-handler detection.

use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::utils::LineIndex;

use super::types::{ExceptionKind, ExceptionMatch, ExceptionStats, ExceptionsResult};

const MAX_SNIPPET: usize = 120;

fn truncate_snippet(s: &str) -> &str {
    s.char_indices()
        .nth(MAX_SNIPPET)
        .map_or(s, |(i, _)| &s[..i])
}

fn get_line_snippet(source: &str, line_1_indexed: usize) -> String {
    source
        .lines()
        .nth(line_1_indexed.saturating_sub(1))
        .unwrap_or("")
        .trim()
        .to_owned()
}

/// Returns `true` when the exception handler body only contains `pass`,
/// ellipsis (`...`), or a bare string literal (non-operational "comment").
fn is_empty_handler_body(body: &[Stmt]) -> bool {
    if body.is_empty() {
        return true;
    }
    body.iter().all(|stmt| match stmt {
        Stmt::Pass(_) => true,
        Stmt::Expr(expr_stmt) => matches!(
            expr_stmt.value.as_ref(),
            Expr::EllipsisLiteral(_) | Expr::StringLiteral(_) | Expr::BytesLiteral(_)
        ),
        _ => false,
    })
}

/// Recursively walks all statements and appends exception-handler anti-patterns.
fn visit_stmts(
    stmts: &[Stmt],
    source: &str,
    file: &Path,
    line_index: &LineIndex,
    out: &mut Vec<ExceptionMatch>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Try(try_stmt) => {
                // Inspect each handler for bare-except and empty bodies.
                for handler in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    let line = line_index.line_index(h.range().start());
                    let snippet = get_line_snippet(source, line);
                    let snippet_display = truncate_snippet(&snippet).to_owned();

                    if h.type_.is_none() {
                        out.push(ExceptionMatch {
                            file: file.to_path_buf(),
                            line,
                            kind: ExceptionKind::BareExcept,
                            description: ExceptionKind::BareExcept.description().to_owned(),
                            snippet: snippet_display.clone(),
                        });
                    } else if is_empty_handler_body(&h.body) {
                        // Only flag EmptyHandler for typed handlers; bare-excepts are
                        // already captured above and must not inflate empty_handler_count.
                        out.push(ExceptionMatch {
                            file: file.to_path_buf(),
                            line,
                            kind: ExceptionKind::EmptyHandler,
                            description: ExceptionKind::EmptyHandler.description().to_owned(),
                            snippet: snippet_display,
                        });
                    }

                    // Recurse into handler body.
                    visit_stmts(&h.body, source, file, line_index, out);
                }
                // Recurse into try/else/finally blocks.
                visit_stmts(&try_stmt.body, source, file, line_index, out);
                visit_stmts(&try_stmt.orelse, source, file, line_index, out);
                visit_stmts(&try_stmt.finalbody, source, file, line_index, out);
            }
            Stmt::FunctionDef(f) => {
                visit_stmts(&f.body, source, file, line_index, out);
            }
            Stmt::ClassDef(c) => {
                visit_stmts(&c.body, source, file, line_index, out);
            }
            Stmt::If(i) => {
                visit_stmts(&i.body, source, file, line_index, out);
                for clause in &i.elif_else_clauses {
                    visit_stmts(&clause.body, source, file, line_index, out);
                }
            }
            Stmt::For(f) => {
                visit_stmts(&f.body, source, file, line_index, out);
                visit_stmts(&f.orelse, source, file, line_index, out);
            }
            Stmt::While(w) => {
                visit_stmts(&w.body, source, file, line_index, out);
                visit_stmts(&w.orelse, source, file, line_index, out);
            }
            Stmt::With(w) => {
                visit_stmts(&w.body, source, file, line_index, out);
            }
            Stmt::Match(m) => {
                for case in &m.cases {
                    visit_stmts(&case.body, source, file, line_index, out);
                }
            }
            _ => {}
        }
    }
}

/// Scans a single Python file's source for exception-handler anti-patterns.
#[must_use]
pub fn detect_python_exceptions(source: &str, file: &Path) -> Vec<ExceptionMatch> {
    let Ok(parsed) = parse_module(source) else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    let mut out = Vec::new();
    visit_stmts(parsed.suite(), source, file, &line_index, &mut out);
    out
}

/// Aggregates per-file results into an [`ExceptionsResult`].
fn aggregate(mut all: Vec<ExceptionMatch>) -> ExceptionsResult {
    all.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
    let mut stats = ExceptionStats::default();
    let mut affected: HashSet<&PathBuf> = HashSet::new();
    for m in &all {
        stats.increment(m.kind);
        affected.insert(&m.file);
    }
    stats.affected_files = affected.len();
    drop(affected); // release borrows before building result
    ExceptionsResult {
        stats,
        matches: all,
    }
}

/// Scans a set of Python source files in parallel for exception anti-patterns.
#[must_use]
pub fn scan_files(paths: &[PathBuf]) -> ExceptionsResult {
    use rayon::prelude::*;

    let all: Vec<ExceptionMatch> = paths
        .par_iter()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e == "py" || e == "pyi")
        })
        .filter_map(|path| {
            let content = std::fs::read_to_string(path).ok()?;
            let matches = detect_python_exceptions(&content, path);
            Some(matches)
        })
        .flatten()
        .collect();

    aggregate(all)
}
