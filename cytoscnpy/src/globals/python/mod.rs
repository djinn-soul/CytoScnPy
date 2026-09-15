//! Python AST visitor for detecting mutable global state.

pub(crate) mod mutations;

use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;
use std::path::Path;

use super::types::{GlobalKind, GlobalMatch};
use crate::utils::LineIndex;
use mutations::detect_function_global_statements;

/// Scans Python source code using the AST to detect mutable globals.
#[must_use]
pub fn detect_python_globals(source: &str, file: &Path) -> Vec<GlobalMatch> {
    let Ok(parsed) = parse_module(source) else {
        return Vec::new();
    };

    let line_index = LineIndex::new(source);
    let mut matches = Vec::new();

    scan_statements(
        parsed.suite(),
        source,
        file,
        &line_index,
        true,
        &mut matches,
    );

    matches
}

fn scan_statements(
    stmts: &[Stmt],
    source: &str,
    file: &Path,
    line_index: &LineIndex,
    is_module_level: bool,
    matches: &mut Vec<GlobalMatch>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Assign(assign) => {
                if is_module_level && is_mutable_collection_expr(&assign.value) {
                    for target in &assign.targets {
                        for name in extract_target_names(target) {
                            let line = line_index.line_index(target.range().start());
                            let col = line_index.column_index(target.range().start());
                            matches.push(GlobalMatch {
                                file: file.to_path_buf(),
                                line,
                                column: col,
                                name,
                                kind: GlobalKind::ModuleCollection,
                                snippet: get_line_snippet(source, line),
                            });
                        }
                    }
                }
            }
            Stmt::AnnAssign(ann) => {
                if is_module_level {
                    if let Some(val) = &ann.value {
                        if is_mutable_collection_expr(val) {
                            for name in extract_target_names(&ann.target) {
                                let line = line_index.line_index(ann.target.range().start());
                                let col = line_index.column_index(ann.target.range().start());
                                matches.push(GlobalMatch {
                                    file: file.to_path_buf(),
                                    line,
                                    column: col,
                                    name,
                                    kind: GlobalKind::ModuleCollection,
                                    snippet: get_line_snippet(source, line),
                                });
                            }
                        }
                    }
                }
            }
            Stmt::ClassDef(class_def) => {
                detect_class_mutable_variables(class_def, source, file, line_index, matches);
            }
            Stmt::FunctionDef(fn_def) => {
                detect_function_global_statements(&fn_def.body, source, file, line_index, matches);
            }
            Stmt::If(if_stmt) => {
                if is_module_level && !is_main_guard(&if_stmt.test) {
                    scan_statements(&if_stmt.body, source, file, line_index, true, matches);
                    for clause in &if_stmt.elif_else_clauses {
                        scan_statements(&clause.body, source, file, line_index, true, matches);
                    }
                }
            }
            Stmt::Try(try_stmt) if is_module_level => {
                scan_statements(&try_stmt.body, source, file, line_index, true, matches);
                for h in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(handler) = h;
                    scan_statements(&handler.body, source, file, line_index, true, matches);
                }
                scan_statements(&try_stmt.orelse, source, file, line_index, true, matches);
                scan_statements(&try_stmt.finalbody, source, file, line_index, true, matches);
            }
            Stmt::Match(m) if is_module_level => {
                for case in &m.cases {
                    scan_statements(&case.body, source, file, line_index, true, matches);
                }
            }
            _ => {}
        }
    }
}

fn is_main_guard(expr: &Expr) -> bool {
    if let Expr::Compare(comp) = expr {
        if let Expr::Name(name) = comp.left.as_ref() {
            if name.id.as_str() == "__name__" {
                return true;
            }
        }
        for right in &comp.comparators {
            if let Expr::Name(name) = right {
                if name.id.as_str() == "__name__" {
                    return true;
                }
            }
        }
    }
    false
}

pub(crate) fn get_line_snippet(source: &str, line_1_indexed: usize) -> String {
    source
        .lines()
        .nth(line_1_indexed.saturating_sub(1))
        .unwrap_or("")
        .trim()
        .to_owned()
}

fn detect_class_mutable_variables(
    class_def: &ast::StmtClassDef,
    source: &str,
    file: &Path,
    line_index: &LineIndex,
    matches: &mut Vec<GlobalMatch>,
) {
    for body_stmt in &class_def.body {
        match body_stmt {
            Stmt::Assign(assign) => {
                if is_mutable_collection_expr(&assign.value) {
                    for target in &assign.targets {
                        for name in extract_target_names(target) {
                            let line = line_index.line_index(target.range().start());
                            let col = line_index.column_index(target.range().start());
                            matches.push(GlobalMatch {
                                file: file.to_path_buf(),
                                line,
                                column: col,
                                name: format!("{}::{}", class_def.name.as_str(), name),
                                kind: GlobalKind::ClassVariable,
                                snippet: get_line_snippet(source, line),
                            });
                        }
                    }
                }
            }
            Stmt::AnnAssign(ann) => {
                if let Some(val) = &ann.value {
                    if is_mutable_collection_expr(val) {
                        for name in extract_target_names(&ann.target) {
                            let line = line_index.line_index(ann.target.range().start());
                            let col = line_index.column_index(ann.target.range().start());
                            matches.push(GlobalMatch {
                                file: file.to_path_buf(),
                                line,
                                column: col,
                                name: format!("{}::{}", class_def.name.as_str(), name),
                                kind: GlobalKind::ClassVariable,
                                snippet: get_line_snippet(source, line),
                            });
                        }
                    }
                }
            }
            Stmt::FunctionDef(fn_def) => {
                detect_function_global_statements(&fn_def.body, source, file, line_index, matches);
            }
            _ => {}
        }
    }
}

pub(crate) fn extract_target_names(expr: &Expr) -> Vec<String> {
    match expr {
        Expr::Name(name) => vec![name.id.as_str().to_owned()],
        Expr::Subscript(s) => extract_target_names(&s.value),
        Expr::Attribute(a) => extract_target_names(&a.value),
        Expr::Tuple(t) => t.elts.iter().flat_map(extract_target_names).collect(),
        Expr::List(l) => l.elts.iter().flat_map(extract_target_names).collect(),
        _ => Vec::new(),
    }
}

/// Checks if an expression creates a mutable collection in Python.
pub(crate) fn is_mutable_collection_expr(expr: &Expr) -> bool {
    match expr {
        Expr::List(_)
        | Expr::Dict(_)
        | Expr::Set(_)
        | Expr::ListComp(_)
        | Expr::DictComp(_)
        | Expr::SetComp(_) => true,
        Expr::Call(call) => match call.func.as_ref() {
            Expr::Name(n) => matches!(
                n.id.as_str(),
                "list"
                    | "dict"
                    | "set"
                    | "bytearray"
                    | "defaultdict"
                    | "deque"
                    | "Counter"
                    | "OrderedDict"
                    | "UserList"
                    | "UserDict"
            ),
            Expr::Attribute(attr) => matches!(
                attr.attr.as_str(),
                "defaultdict"
                    | "deque"
                    | "Counter"
                    | "bytearray"
                    | "OrderedDict"
                    | "UserList"
                    | "UserDict"
            ),
            _ => false,
        },
        _ => false,
    }
}
