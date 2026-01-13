//! Intraprocedural taint analysis.
//!
//! Analyzes data flow within a single function.

use super::propagation::{is_expr_tainted, is_parameterized_query, is_sanitizer_call, TaintState};
use super::sinks::{check_sink, SinkInfo};
use super::sources::check_taint_source;
use super::types::{TaintFinding, TaintInfo};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;
use std::path::Path;

/// Performs intraprocedural taint analysis on a function.
pub fn analyze_function(
    func: &ast::StmtFunctionDef,
    file_path: &Path,
    initial_taint: Option<TaintState>,
) -> Vec<TaintFinding> {
    let mut state = initial_taint.unwrap_or_default();
    let mut findings = Vec::new();

    // Analyze each statement in the function body
    for stmt in &func.body {
        analyze_stmt(stmt, &mut state, &mut findings, file_path);
    }

    findings
}

/// Analyzes an async function.
pub fn analyze_async_function(
    func: &ast::StmtFunctionDef,
    file_path: &Path,
    initial_taint: Option<TaintState>,
) -> Vec<TaintFinding> {
    let mut state = initial_taint.unwrap_or_default();
    let mut findings = Vec::new();

    for stmt in &func.body {
        analyze_stmt(stmt, &mut state, &mut findings, file_path);
    }

    findings
}

/// Public wrapper for analyzing a single statement.
/// Used for module-level statement analysis.
pub fn analyze_stmt_public(
    stmt: &Stmt,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
) {
    analyze_stmt(stmt, state, findings, file_path);
}

/// Analyzes a statement for taint flow.
#[allow(clippy::too_many_lines)]
fn analyze_stmt(
    stmt: &Stmt,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
) {
    match stmt {
        Stmt::Assign(assign) => handle_assign(assign, state, findings, file_path),
        Stmt::AnnAssign(assign) => handle_ann_assign(assign, state, findings, file_path),
        Stmt::AugAssign(assign) => handle_aug_assign(assign, state, findings, file_path),
        Stmt::Expr(expr_stmt) => {
            check_expr_for_sinks(&expr_stmt.value, state, findings, file_path);
        }
        Stmt::Return(ret) => {
            if let Some(value) = &ret.value {
                check_expr_for_sinks(value, state, findings, file_path);
            }
        }
        Stmt::If(if_stmt) => handle_if(if_stmt, state, findings, file_path),
        Stmt::For(for_stmt) => handle_for(for_stmt, state, findings, file_path),
        Stmt::While(while_stmt) => handle_while(while_stmt, state, findings, file_path),
        Stmt::With(with_stmt) => {
            for s in &with_stmt.body {
                analyze_stmt(s, state, findings, file_path);
            }
        }
        Stmt::Try(try_stmt) => handle_try(try_stmt, state, findings, file_path),
        Stmt::FunctionDef(nested_func) => {
            handle_function_def(nested_func, state, findings, file_path);
        }
        _ => {}
    }
}

fn handle_assign(
    assign: &ast::StmtAssign,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
) {
    check_expr_for_sinks(&assign.value, state, findings, file_path);

    if let Some(taint_info) = check_taint_source(&assign.value) {
        for target in &assign.targets {
            if let Expr::Name(name) = target {
                state.mark_tainted(name.id.as_str(), taint_info.clone());
            }
        }
    } else if let Some(taint_info) = is_expr_tainted(&assign.value, state) {
        if let Expr::Call(call) = &*assign.value {
            if is_sanitizer_call(call) {
                for target in &assign.targets {
                    if let Expr::Name(name) = target {
                        state.sanitize(name.id.as_str());
                    }
                }
                return;
            }
        }
        for target in &assign.targets {
            if let Expr::Name(name) = target {
                state.mark_tainted(name.id.as_str(), taint_info.extend_path(name.id.as_str()));
            }
        }
    }
}

fn handle_ann_assign(
    assign: &ast::StmtAnnAssign,
    state: &mut TaintState,
    _findings: &mut Vec<TaintFinding>,
    _file_path: &Path,
) {
    if let Some(value) = &assign.value {
        if let Some(taint_info) = check_taint_source(value) {
            if let Expr::Name(name) = &*assign.target {
                state.mark_tainted(name.id.as_str(), taint_info);
            }
        } else if let Some(taint_info) = is_expr_tainted(value, state) {
            if let Expr::Call(call) = &**value {
                if is_sanitizer_call(call) {
                    if let Expr::Name(name) = &*assign.target {
                        state.sanitize(name.id.as_str());
                    }
                    return;
                }
            }
            if let Expr::Name(name) = &*assign.target {
                state.mark_tainted(name.id.as_str(), taint_info.extend_path(name.id.as_str()));
            }
        }
    }
}

fn handle_aug_assign(
    assign: &ast::StmtAugAssign,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
) {
    if let Some(taint_info) = is_expr_tainted(&assign.value, state) {
        if let Expr::Name(name) = &*assign.target {
            state.mark_tainted(name.id.as_str(), taint_info.extend_path(name.id.as_str()));
        }
    }
    check_expr_for_sinks(&assign.value, state, findings, file_path);
}

fn handle_if(
    if_stmt: &ast::StmtIf,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
) {
    check_expr_for_sinks(&if_stmt.test, state, findings, file_path);

    let mut then_state = state.clone();

    for s in &if_stmt.body {
        analyze_stmt(s, &mut then_state, findings, file_path);
    }

    let mut combined_state = then_state;

    for clause in &if_stmt.elif_else_clauses {
        let mut clause_state = state.clone();
        if let Some(test) = &clause.test {
            check_expr_for_sinks(test, state, findings, file_path);
        }
        for s in &clause.body {
            analyze_stmt(s, &mut clause_state, findings, file_path);
        }
        combined_state.merge(&clause_state);
    }

    *state = combined_state;
}

fn handle_for(
    for_stmt: &ast::StmtFor,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
) {
    if let Some(taint_info) = is_expr_tainted(&for_stmt.iter, state) {
        if let Expr::Name(name) = &*for_stmt.target {
            state.mark_tainted(name.id.as_str(), taint_info);
        }
    }

    for s in &for_stmt.body {
        analyze_stmt(s, state, findings, file_path);
    }
    for s in &for_stmt.orelse {
        analyze_stmt(s, state, findings, file_path);
    }
}

fn handle_while(
    while_stmt: &ast::StmtWhile,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
) {
    check_expr_for_sinks(&while_stmt.test, state, findings, file_path);

    for s in &while_stmt.body {
        analyze_stmt(s, state, findings, file_path);
    }
    for s in &while_stmt.orelse {
        analyze_stmt(s, state, findings, file_path);
    }
}

fn handle_try(
    try_stmt: &ast::StmtTry,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
) {
    for s in &try_stmt.body {
        analyze_stmt(s, state, findings, file_path);
    }
    for handler in &try_stmt.handlers {
        let ast::ExceptHandler::ExceptHandler(h) = handler;
        for s in &h.body {
            analyze_stmt(s, state, findings, file_path);
        }
    }
    for s in &try_stmt.orelse {
        analyze_stmt(s, state, findings, file_path);
    }
    for s in &try_stmt.finalbody {
        analyze_stmt(s, state, findings, file_path);
    }
}

fn handle_function_def(
    nested_func: &ast::StmtFunctionDef,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
) {
    if nested_func.is_async {
        let nested_findings = analyze_async_function(nested_func, file_path, Some(state.clone()));
        findings.extend(nested_findings);
    } else {
        let nested_findings = analyze_function(nested_func, file_path, Some(state.clone()));
        findings.extend(nested_findings);
    }
}

fn handle_call_sink(
    call: &ast::ExprCall,
    state: &TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
) {
    // Check if this call is a sink
    if let Some(sink_info) = check_sink(call) {
        // Check if any dangerous argument is tainted
        for arg_idx in &sink_info.dangerous_args {
            if let Some(arg) = call.arguments.args.get(*arg_idx) {
                if let Some(taint_info) = is_expr_tainted(arg, state) {
                    // Check for sanitization (e.g., parameterized queries)
                    if sink_info.vuln_type == super::types::VulnType::SqlInjection
                        && is_parameterized_query(call)
                    {
                        continue;
                    }

                    let finding = create_finding(
                        &taint_info,
                        &sink_info,
                        call.range().start().to_u32() as usize,
                        file_path,
                    );
                    findings.push(finding);
                }
            }
        }
    }

    // Recursively check arguments
    for arg in &call.arguments.args {
        check_expr_for_sinks(arg, state, findings, file_path);
    }
}

/// Checks an expression for dangerous sink calls.
fn check_expr_for_sinks(
    expr: &Expr,
    state: &TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
) {
    match expr {
        Expr::Call(call) => handle_call_sink(call, state, findings, file_path),

        Expr::BinOp(binop) => {
            check_expr_for_sinks(&binop.left, state, findings, file_path);
            check_expr_for_sinks(&binop.right, state, findings, file_path);
        }

        Expr::If(ifexp) => {
            check_expr_for_sinks(&ifexp.test, state, findings, file_path);
            check_expr_for_sinks(&ifexp.body, state, findings, file_path);
            check_expr_for_sinks(&ifexp.orelse, state, findings, file_path);
        }

        Expr::List(list) => {
            for elt in &list.elts {
                check_expr_for_sinks(elt, state, findings, file_path);
            }
        }

        Expr::ListComp(comp) => {
            check_expr_for_sinks(&comp.elt, state, findings, file_path);
        }

        _ => {}
    }
}

/// Creates a taint finding from source and sink info.
fn create_finding(
    taint_info: &TaintInfo,
    sink_info: &SinkInfo,
    sink_line: usize,
    file_path: &Path,
) -> TaintFinding {
    TaintFinding {
        source: taint_info.source.to_string(),
        source_line: taint_info.source_line,
        sink: sink_info.name.clone(),
        sink_line,
        sink_col: 0,
        flow_path: taint_info.path.clone(),
        vuln_type: sink_info.vuln_type.clone(),
        severity: sink_info.severity,
        file: file_path.to_path_buf(),
        remediation: sink_info.remediation.clone(),
    }
}
