//! Global keyword mutation detection in Python functions.

use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::Ranged;
use std::collections::HashSet;
use std::path::Path;

use super::super::types::{GlobalKind, GlobalMatch};
use super::{extract_target_names, get_line_snippet};
use crate::utils::LineIndex;

/// Detects mutations of variables declared as `global` inside a function body.
pub(crate) fn detect_function_global_statements(
    body: &[Stmt],
    source: &str,
    file: &Path,
    line_index: &LineIndex,
    matches: &mut Vec<GlobalMatch>,
) {
    let mut declared_globals = HashSet::new();
    collect_declared_globals(body, &mut declared_globals);

    if declared_globals.is_empty() {
        return;
    }

    find_global_mutations(body, &declared_globals, source, file, line_index, matches);
}

fn collect_declared_globals(stmts: &[Stmt], declared: &mut HashSet<String>) {
    for stmt in stmts {
        match stmt {
            Stmt::Global(g) => {
                for ident in &g.names {
                    declared.insert(ident.as_str().to_owned());
                }
            }
            Stmt::If(i) => {
                collect_declared_globals(&i.body, declared);
                for clause in &i.elif_else_clauses {
                    collect_declared_globals(&clause.body, declared);
                }
            }
            Stmt::Try(t) => {
                collect_declared_globals(&t.body, declared);
                for h in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(handler) = h;
                    collect_declared_globals(&handler.body, declared);
                }
                collect_declared_globals(&t.orelse, declared);
                collect_declared_globals(&t.finalbody, declared);
            }
            Stmt::For(f) => {
                collect_declared_globals(&f.body, declared);
                collect_declared_globals(&f.orelse, declared);
            }
            Stmt::While(w) => {
                collect_declared_globals(&w.body, declared);
                collect_declared_globals(&w.orelse, declared);
            }
            Stmt::With(w) => {
                collect_declared_globals(&w.body, declared);
            }
            Stmt::Match(m) => {
                for case in &m.cases {
                    collect_declared_globals(&case.body, declared);
                }
            }
            _ => {}
        }
    }
}

fn find_global_mutations(
    stmts: &[Stmt],
    declared: &HashSet<String>,
    source: &str,
    file: &Path,
    line_index: &LineIndex,
    matches: &mut Vec<GlobalMatch>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Assign(assign) => {
                for target in &assign.targets {
                    record_matching_targets(target, declared, source, file, line_index, matches);
                }
            }
            Stmt::AugAssign(aug) => {
                record_matching_targets(&aug.target, declared, source, file, line_index, matches);
            }
            Stmt::Delete(del_stmt) => {
                for target in &del_stmt.targets {
                    record_matching_targets(target, declared, source, file, line_index, matches);
                }
            }
            Stmt::If(i) => {
                find_global_mutations(&i.body, declared, source, file, line_index, matches);
                for clause in &i.elif_else_clauses {
                    find_global_mutations(
                        &clause.body,
                        declared,
                        source,
                        file,
                        line_index,
                        matches,
                    );
                }
            }
            Stmt::Try(t) => {
                find_global_mutations(&t.body, declared, source, file, line_index, matches);
                for h in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(handler) = h;
                    find_global_mutations(
                        &handler.body,
                        declared,
                        source,
                        file,
                        line_index,
                        matches,
                    );
                }
                find_global_mutations(&t.orelse, declared, source, file, line_index, matches);
                find_global_mutations(&t.finalbody, declared, source, file, line_index, matches);
            }
            Stmt::For(f) => {
                find_global_mutations(&f.body, declared, source, file, line_index, matches);
                find_global_mutations(&f.orelse, declared, source, file, line_index, matches);
            }
            Stmt::While(w) => {
                find_global_mutations(&w.body, declared, source, file, line_index, matches);
                find_global_mutations(&w.orelse, declared, source, file, line_index, matches);
            }
            Stmt::With(w) => {
                find_global_mutations(&w.body, declared, source, file, line_index, matches);
            }
            Stmt::Match(m) => {
                for case in &m.cases {
                    find_global_mutations(&case.body, declared, source, file, line_index, matches);
                }
            }
            _ => {}
        }
    }
}

fn record_matching_targets(
    target: &ruff_python_ast::Expr,
    declared: &HashSet<String>,
    source: &str,
    file: &Path,
    line_index: &LineIndex,
    matches: &mut Vec<GlobalMatch>,
) {
    for name in extract_target_names(target) {
        if declared.contains(&name) {
            let line = line_index.line_index(target.range().start());
            let col = line_index.column_index(target.range().start());
            matches.push(GlobalMatch {
                file: file.to_path_buf(),
                line,
                column: col,
                name,
                kind: GlobalKind::GlobalMutation,
                snippet: get_line_snippet(source, line),
            });
        }
    }
}
