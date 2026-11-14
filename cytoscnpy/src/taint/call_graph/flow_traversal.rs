//! Control-flow, context, match and exception statement traversal for call graph.
//!
//! Isolates compound control-flow traversal from statement definitions and
//! assignments, keeping statement visitor modules focused and strictly modular.

use super::statement_traversal::visit_stmts;
use super::CallGraph;
use ruff_python_ast::{self as ast, Stmt};

pub(super) fn visit_branch_stmt(
    graph: &mut CallGraph,
    stmt: &Stmt,
    current_func: Option<&str>,
    module_name: &str,
) -> bool {
    match stmt {
        Stmt::If(if_stmt) => {
            if let Some(caller) = current_func {
                graph.visit_expr_for_calls(&if_stmt.test, caller, module_name);
            }
            visit_stmts(graph, &if_stmt.body, current_func, module_name);
            for elif in &if_stmt.elif_else_clauses {
                if let (Some(caller), Some(test)) = (current_func, &elif.test) {
                    graph.visit_expr_for_calls(test, caller, module_name);
                }
                visit_stmts(graph, &elif.body, current_func, module_name);
            }
        }
        Stmt::For(for_stmt) => {
            if let Some(caller) = current_func {
                graph.visit_expr_for_calls(&for_stmt.iter, caller, module_name);
                graph.visit_expr_for_calls(&for_stmt.target, caller, module_name);
            }
            visit_stmts(graph, &for_stmt.body, current_func, module_name);
            visit_stmts(graph, &for_stmt.orelse, current_func, module_name);
        }
        Stmt::While(while_stmt) => {
            if let Some(caller) = current_func {
                graph.visit_expr_for_calls(&while_stmt.test, caller, module_name);
            }
            visit_stmts(graph, &while_stmt.body, current_func, module_name);
            visit_stmts(graph, &while_stmt.orelse, current_func, module_name);
        }
        _ => return false,
    }
    true
}

pub(super) fn visit_context_stmt(
    graph: &mut CallGraph,
    stmt: &Stmt,
    current_func: Option<&str>,
    module_name: &str,
) -> bool {
    match stmt {
        Stmt::With(with_stmt) => {
            if let Some(caller) = current_func {
                for item in &with_stmt.items {
                    graph.visit_expr_for_calls(&item.context_expr, caller, module_name);
                }
            }
            visit_stmts(graph, &with_stmt.body, current_func, module_name);
        }
        Stmt::Match(match_stmt) => {
            if let Some(caller) = current_func {
                graph.visit_expr_for_calls(&match_stmt.subject, caller, module_name);
            }
            visit_match_cases(graph, &match_stmt.cases, current_func, module_name);
        }
        _ => return false,
    }
    true
}

fn visit_match_cases(
    graph: &mut CallGraph,
    cases: &[ast::MatchCase],
    current_func: Option<&str>,
    module_name: &str,
) {
    for case in cases {
        if let Some(caller) = current_func {
            if let Some(guard) = &case.guard {
                graph.visit_expr_for_calls(guard, caller, module_name);
            }
            graph.visit_pattern_for_calls(&case.pattern, caller, module_name);
        }
        visit_stmts(graph, &case.body, current_func, module_name);
    }
}

pub(super) fn visit_exception_stmt(
    graph: &mut CallGraph,
    stmt: &Stmt,
    current_func: Option<&str>,
    module_name: &str,
) -> bool {
    let Stmt::Try(try_stmt) = stmt else {
        return false;
    };

    visit_stmts(graph, &try_stmt.body, current_func, module_name);
    for handler in &try_stmt.handlers {
        let ast::ExceptHandler::ExceptHandler(h) = handler;
        if let (Some(caller), Some(type_)) = (current_func, h.type_.as_deref()) {
            graph.visit_expr_for_calls(type_, caller, module_name);
        }
        visit_stmts(graph, &h.body, current_func, module_name);
    }
    visit_stmts(graph, &try_stmt.orelse, current_func, module_name);
    visit_stmts(graph, &try_stmt.finalbody, current_func, module_name);
    true
}
