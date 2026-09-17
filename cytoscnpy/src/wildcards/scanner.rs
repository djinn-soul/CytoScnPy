//! Python AST scanner for wildcard-import (`from module import *`) detection.

use ruff_python_ast::Stmt;
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::utils::LineIndex;

use super::types::{WildcardMatch, WildcardStats, WildcardsResult};

const MAX_SNIPPET: usize = 120;

fn get_line_snippet(source: &str, line_1_indexed: usize) -> String {
    source
        .lines()
        .nth(line_1_indexed.saturating_sub(1))
        .unwrap_or("")
        .trim()
        .chars()
        .take(MAX_SNIPPET)
        .collect()
}

/// Recursively walks all statements and collects `from … import *` occurrences.
fn visit_stmts(
    stmts: &[Stmt],
    source: &str,
    file: &Path,
    line_index: &LineIndex,
    out: &mut Vec<WildcardMatch>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::ImportFrom(import_from) => {
                // A star-import has exactly one name entry whose `name` is `"*"`.
                let is_star = import_from
                    .names
                    .iter()
                    .any(|alias| alias.name.as_str() == "*");
                if is_star {
                    let line = line_index.line_index(import_from.range().start());
                    let base_module = import_from
                        .module
                        .as_ref()
                        .map(ruff_python_ast::Identifier::as_str)
                        .unwrap_or("");
                    let module = if import_from.level > 0 {
                        format!("{}{}", ".".repeat(import_from.level as usize), base_module)
                    } else if base_module.is_empty() {
                        "<unknown>".to_owned()
                    } else {
                        base_module.to_owned()
                    };
                    out.push(WildcardMatch {
                        file: file.to_path_buf(),
                        line,
                        module,
                        snippet: get_line_snippet(source, line),
                    });
                }
            }
            // Recurse into nested scopes so `from x import *` inside functions / classes
            // is also detected, not just module-level imports.
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
            Stmt::Try(t) => {
                visit_stmts(&t.body, source, file, line_index, out);
                for h in &t.handlers {
                    let ruff_python_ast::ExceptHandler::ExceptHandler(handler) = h;
                    visit_stmts(&handler.body, source, file, line_index, out);
                }
                visit_stmts(&t.orelse, source, file, line_index, out);
                visit_stmts(&t.finalbody, source, file, line_index, out);
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

/// Scans a single Python source file for wildcard imports.
#[must_use]
pub fn detect_wildcard_imports(source: &str, file: &Path) -> Vec<WildcardMatch> {
    let Ok(parsed) = parse_module(source) else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    let mut out = Vec::new();
    visit_stmts(parsed.suite(), source, file, &line_index, &mut out);
    out
}

/// Aggregates per-file results into a [`WildcardsResult`].
fn aggregate(mut all: Vec<WildcardMatch>) -> WildcardsResult {
    all.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
    let mut stats = WildcardStats::default();
    let mut affected: HashSet<&PathBuf> = HashSet::new();
    for m in &all {
        stats.increment();
        affected.insert(&m.file);
    }
    stats.affected_files = affected.len();
    WildcardsResult {
        stats,
        matches: all,
    }
}

/// Scans a set of Python source files in parallel for wildcard imports.
#[must_use]
pub fn scan_files(paths: &[PathBuf]) -> WildcardsResult {
    use rayon::prelude::*;

    let all: Vec<WildcardMatch> = paths
        .par_iter()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e == "py" || e == "pyi")
        })
        .filter_map(|path| {
            let content = std::fs::read_to_string(path).ok()?;
            Some(detect_wildcard_imports(&content, path))
        })
        .flatten()
        .collect();

    aggregate(all)
}
