//! Python AST visitor for detecting module-level import-time side effects.

use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;
use std::path::Path;

use super::types::{SideEffectKind, SideEffectMatch};
use crate::utils::LineIndex;

const MAX_SNIPPET: usize = 120;

/// Detects import-time module-level side effects in Python code.
#[must_use]
pub fn detect_python_side_effects(source: &str, file: &Path) -> Vec<SideEffectMatch> {
    if is_exempt_python_entrypoint(file) {
        return Vec::new();
    }

    let Ok(parsed) = parse_module(source) else {
        return Vec::new();
    };

    let line_index = LineIndex::new(source);
    let mut matches = Vec::new();

    scan_module_statements(parsed.suite(), source, file, &line_index, &mut matches);

    matches
}

fn scan_module_statements(
    stmts: &[Stmt],
    source: &str,
    file: &Path,
    line_index: &LineIndex,
    matches: &mut Vec<SideEffectMatch>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Expr(expr_stmt) => {
                if let Expr::Call(call) = expr_stmt.value.as_ref() {
                    if !is_safe_toplevel_call(&call.func) {
                        let line = line_index.line_index(expr_stmt.range().start());
                        let col = line_index.column_index(expr_stmt.range().start());
                        matches.push(SideEffectMatch {
                            file: file.to_path_buf(),
                            line,
                            column: col,
                            kind: SideEffectKind::PythonTopLevelCall,
                            snippet: get_line_snippet(source, line),
                        });
                    }
                }
            }
            Stmt::For(for_stmt) => {
                let line = line_index.line_index(for_stmt.range().start());
                let col = line_index.column_index(for_stmt.range().start());
                matches.push(SideEffectMatch {
                    file: file.to_path_buf(),
                    line,
                    column: col,
                    kind: SideEffectKind::PythonTopLevelLoop,
                    snippet: get_line_snippet(source, line),
                });
            }
            Stmt::While(while_stmt) => {
                let line = line_index.line_index(while_stmt.range().start());
                let col = line_index.column_index(while_stmt.range().start());
                matches.push(SideEffectMatch {
                    file: file.to_path_buf(),
                    line,
                    column: col,
                    kind: SideEffectKind::PythonTopLevelLoop,
                    snippet: get_line_snippet(source, line),
                });
            }
            Stmt::With(with_stmt) => {
                let line = line_index.line_index(with_stmt.range().start());
                let col = line_index.column_index(with_stmt.range().start());
                matches.push(SideEffectMatch {
                    file: file.to_path_buf(),
                    line,
                    column: col,
                    kind: SideEffectKind::PythonTopLevelWith,
                    snippet: get_line_snippet(source, line),
                });
            }
            Stmt::If(if_stmt) => {
                if !is_main_guard(&if_stmt.test) && !is_type_checking_guard(&if_stmt.test) {
                    scan_module_statements(&if_stmt.body, source, file, line_index, matches);
                    for clause in &if_stmt.elif_else_clauses {
                        scan_module_statements(&clause.body, source, file, line_index, matches);
                    }
                }
            }
            Stmt::Try(try_stmt) => {
                scan_module_statements(&try_stmt.body, source, file, line_index, matches);
                for h in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(handler) = h;
                    scan_module_statements(&handler.body, source, file, line_index, matches);
                }
                scan_module_statements(&try_stmt.orelse, source, file, line_index, matches);
                scan_module_statements(&try_stmt.finalbody, source, file, line_index, matches);
            }
            // Functions, classes, type declarations, imports, etc. are inert definitions.
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

fn is_type_checking_guard(expr: &Expr) -> bool {
    match expr {
        Expr::Name(n) => n.id.as_str() == "TYPE_CHECKING",
        Expr::Attribute(a) => a.attr.as_str() == "TYPE_CHECKING",
        _ => false,
    }
}

fn is_safe_toplevel_call(func: &Expr) -> bool {
    match func {
        Expr::Name(n) => matches!(
            n.id.as_str(),
            "print" | "register" | "TypeVar" | "NewType" | "NamedTuple" | "cast" | "setup"
        ),
        Expr::Attribute(a) => {
            let attr = a.attr.as_str();
            matches!(
                attr,
                "debug"
                    | "info"
                    | "warning"
                    | "error"
                    | "critical"
                    | "warn"
                    | "log"
                    | "filterwarnings"
                    | "simplefilter"
                    | "catch_warnings"
                    | "getLogger"
                    | "get_logger"
                    | "basicConfig"
            ) || is_safe_namespace_target(&a.value)
        }
        _ => false,
    }
}

fn is_safe_namespace_target(expr: &Expr) -> bool {
    match expr {
        Expr::Name(n) => matches!(n.id.as_str(), "logging" | "warnings" | "logger" | "sys"),
        _ => false,
    }
}

fn is_exempt_python_entrypoint(file: &Path) -> bool {
    if let Some(file_name) = file.file_name().and_then(|n| n.to_str()) {
        let lower = file_name.to_lowercase();
        return lower == "setup.py"
            || lower == "manage.py"
            || lower == "wsgi.py"
            || lower == "asgi.py"
            || lower == "conftest.py";
    }
    false
}

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
