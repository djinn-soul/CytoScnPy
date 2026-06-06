use super::{CallGraph, CallGraphNode};
use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

pub(super) fn visit_stmt(
    graph: &mut CallGraph,
    stmt: &Stmt,
    current_func: Option<&str>,
    module_name: &str,
) {
    if visit_definition_stmt(graph, stmt, current_func, module_name) {
        return;
    }
    if visit_assignment_stmt(graph, stmt, current_func, module_name) {
        return;
    }
    if visit_simple_expr_stmt(graph, stmt, current_func, module_name) {
        return;
    }
    if visit_branch_stmt(graph, stmt, current_func, module_name) {
        return;
    }
    if visit_context_stmt(graph, stmt, current_func, module_name) {
        return;
    }
    let _ = visit_exception_stmt(graph, stmt, current_func, module_name);
}

fn visit_definition_stmt(
    graph: &mut CallGraph,
    stmt: &Stmt,
    current_func: Option<&str>,
    module_name: &str,
) -> bool {
    match stmt {
        Stmt::FunctionDef(func) => {
            let func_name = graph.get_qualified_name(&func.name, module_name);
            graph.nodes.insert(
                func_name.clone(),
                CallGraphNode {
                    name: func_name.clone(),
                    line: func.range().start().to_u32() as usize,
                    calls: FxHashSet::default(),
                    called_by: FxHashSet::default(),
                    params: CallGraph::extract_params(&func.parameters),
                    is_root: false,
                },
            );
            visit_stmts(graph, &func.body, Some(&func_name), module_name);
        }
        Stmt::ClassDef(class) => {
            graph.class_stack.push(class.name.to_string());
            visit_stmts(graph, &class.body, current_func, module_name);
            graph.class_stack.pop();
        }
        _ => return false,
    }
    true
}

fn visit_assignment_stmt(
    graph: &mut CallGraph,
    stmt: &Stmt,
    current_func: Option<&str>,
    module_name: &str,
) -> bool {
    let Some(caller) = current_func else {
        return false;
    };

    match stmt {
        Stmt::Assign(assign) => {
            graph.visit_expr_for_calls(&assign.value, caller, module_name);
            for target in &assign.targets {
                graph.visit_expr_for_calls(target, caller, module_name);
            }
        }
        Stmt::AugAssign(aug_assign) => {
            graph.visit_expr_for_calls(&aug_assign.value, caller, module_name);
            graph.visit_expr_for_calls(&aug_assign.target, caller, module_name);
        }
        Stmt::AnnAssign(ann_assign) => {
            if let Some(value) = &ann_assign.value {
                graph.visit_expr_for_calls(value, caller, module_name);
            }
        }
        _ => return false,
    }
    true
}

fn visit_simple_expr_stmt(
    graph: &mut CallGraph,
    stmt: &Stmt,
    current_func: Option<&str>,
    module_name: &str,
) -> bool {
    let Some(caller) = current_func else {
        return false;
    };

    match stmt {
        Stmt::Expr(expr_stmt) => graph.visit_expr_for_calls(&expr_stmt.value, caller, module_name),
        Stmt::Assert(assert_stmt) => {
            graph.visit_expr_for_calls(&assert_stmt.test, caller, module_name);
            if let Some(msg) = &assert_stmt.msg {
                graph.visit_expr_for_calls(msg, caller, module_name);
            }
        }
        Stmt::Return(ret) => {
            if let Some(value) = &ret.value {
                graph.visit_expr_for_calls(value, caller, module_name);
            }
        }
        Stmt::Raise(raise_stmt) => {
            if let Some(exc) = &raise_stmt.exc {
                graph.visit_expr_for_calls(exc, caller, module_name);
            }
        }
        _ => return false,
    }
    true
}

fn visit_branch_stmt(
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
            for clause in &if_stmt.elif_else_clauses {
                if let (Some(caller), Some(test)) = (current_func, clause.test.as_ref()) {
                    graph.visit_expr_for_calls(test, caller, module_name);
                }
                visit_stmts(graph, &clause.body, current_func, module_name);
            }
        }
        Stmt::For(for_stmt) => {
            if let Some(caller) = current_func {
                graph.visit_expr_for_calls(&for_stmt.iter, caller, module_name);
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

fn visit_context_stmt(
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

fn visit_exception_stmt(
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

fn visit_stmts(
    graph: &mut CallGraph,
    stmts: &[Stmt],
    current_func: Option<&str>,
    module_name: &str,
) {
    for stmt in stmts {
        visit_stmt(graph, stmt, current_func, module_name);
    }
}
