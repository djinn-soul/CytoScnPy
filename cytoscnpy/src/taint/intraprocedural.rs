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
fn analyze_stmt(
    stmt: &Stmt,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
) {
    match stmt {
        // Assignment: x = expr
        Stmt::Assign(assign) => {
            // First, check RHS for sinks with current taint state
            check_expr_for_sinks(&assign.value, state, findings, file_path);

            // Check if right-hand side is a taint source
            if let Some(taint_info) = check_taint_source(&assign.value) {
                for target in &assign.targets {
                    if let Expr::Name(name) = target {
                        state.mark_tainted(name.id.as_str(), taint_info.clone());
                    }
                }
            }
            // Check if right-hand side contains tainted values
            else if let Some(taint_info) = is_expr_tainted(&assign.value, state) {
                // Check if this is a sanitizer
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
                // Propagate taint
                for target in &assign.targets {
                    if let Expr::Name(name) = target {
                        state.mark_tainted(
                            name.id.as_str(),
                            taint_info.extend_path(name.id.as_str()),
                        );
                    }
                }
            }
        }

        // Annotated assignment: x: int = expr
        Stmt::AnnAssign(assign) => {
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
                        state.mark_tainted(
                            name.id.as_str(),
                            taint_info.extend_path(name.id.as_str()),
                        );
                    }
                }
            }
        }

        // Augmented assignment: x += expr
        Stmt::AugAssign(assign) => {
            // Check if the RHS is tainted
            if let Some(taint_info) = is_expr_tainted(&assign.value, state) {
                if let Expr::Name(name) = &*assign.target {
                    // Propagate taint to the target
                    state.mark_tainted(name.id.as_str(), taint_info.extend_path(name.id.as_str()));
                }
            }
            // Also, if the target itself is already tainted, it remains tainted
            // and we should check the RHS for sinks
            check_expr_for_sinks(&assign.value, state, findings, file_path);
        }

        // Expression statement: expr (often a call)
        Stmt::Expr(expr_stmt) => {
            check_expr_for_sinks(&expr_stmt.value, state, findings, file_path);
        }

        // Return statement
        Stmt::Return(ret) => {
            if let Some(value) = &ret.value {
                check_expr_for_sinks(value, state, findings, file_path);
            }
        }

        // If statement
        Stmt::If(if_stmt) => {
            // Analyze condition
            check_expr_for_sinks(&if_stmt.test, state, findings, file_path);

            // Clone state for branch analysis
            let mut then_state = state.clone();

            for s in &if_stmt.body {
                analyze_stmt(s, &mut then_state, findings, file_path);
            }

            // Handle elif/else clauses
            // We need to merge states from all branches
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

            // Update main state
            *state = combined_state;
        }

        // For loop
        Stmt::For(for_stmt) => {
            // Check iterator
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

        // While loop
        Stmt::While(while_stmt) => {
            check_expr_for_sinks(&while_stmt.test, state, findings, file_path);

            for s in &while_stmt.body {
                analyze_stmt(s, state, findings, file_path);
            }
            for s in &while_stmt.orelse {
                analyze_stmt(s, state, findings, file_path);
            }
        }

        // With statement
        Stmt::With(with_stmt) => {
            for s in &with_stmt.body {
                analyze_stmt(s, state, findings, file_path);
            }
        }

        // Try statement
        Stmt::Try(try_stmt) => {
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

        // Nested function - analyze separately
        Stmt::FunctionDef(nested_func) => {
            if nested_func.is_async {
                let nested_findings =
                    analyze_async_function(nested_func, file_path, Some(state.clone()));
                findings.extend(nested_findings);
            } else {
                let nested_findings = analyze_function(nested_func, file_path, Some(state.clone()));
                findings.extend(nested_findings);
            }
        }

        _ => {}
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
        Expr::Call(call) => {
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
