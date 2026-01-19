//! Intraprocedural taint analysis.
//!
//! Analyzes data flow within a single function.

use super::analyzer::TaintAnalyzer;
use super::propagation::{is_expr_tainted, is_parameterized_query, is_sanitizer_call, TaintState};
use super::types::{SinkMatch, TaintFinding, TaintInfo};
use crate::utils::LineIndex;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;
use std::path::Path;

/// Performs intraprocedural taint analysis on a function.
pub fn analyze_function(
    func: &ast::StmtFunctionDef,
    analyzer: &TaintAnalyzer,
    file_path: &Path,
    line_index: &LineIndex,
    initial_taint: Option<TaintState>,
) -> Vec<TaintFinding> {
    let mut state = initial_taint.unwrap_or_default();
    let mut findings = Vec::new();

    // Analyze each statement in the function body
    for stmt in &func.body {
        analyze_stmt(
            stmt,
            analyzer,
            &mut state,
            &mut findings,
            file_path,
            line_index,
        );
    }

    findings
}

/// Analyzes an async function.
pub fn analyze_async_function(
    func: &ast::StmtFunctionDef,
    analyzer: &TaintAnalyzer,
    file_path: &Path,
    line_index: &LineIndex,
    initial_taint: Option<TaintState>,
) -> Vec<TaintFinding> {
    let mut state = initial_taint.unwrap_or_default();
    let mut findings = Vec::new();

    for stmt in &func.body {
        analyze_stmt(
            stmt,
            analyzer,
            &mut state,
            &mut findings,
            file_path,
            line_index,
        );
    }

    findings
}

/// Public wrapper for analyzing a single statement.
/// Used for module-level statement analysis.
pub fn analyze_stmt_public(
    stmt: &Stmt,
    analyzer: &TaintAnalyzer,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
    line_index: &LineIndex,
) {
    analyze_stmt(stmt, analyzer, state, findings, file_path, line_index);
}

/// Analyzes a statement for taint flow.
#[allow(clippy::too_many_lines)]
fn analyze_stmt(
    stmt: &Stmt,
    analyzer: &TaintAnalyzer,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
    line_index: &LineIndex,
) {
    match stmt {
        Stmt::Assign(assign) => {
            handle_assign(assign, analyzer, state, findings, file_path, line_index);
        }
        Stmt::AnnAssign(assign) => {
            handle_ann_assign(assign, analyzer, state, findings, file_path, line_index);
        }
        Stmt::AugAssign(assign) => {
            handle_aug_assign(assign, analyzer, state, findings, file_path, line_index);
        }
        Stmt::Expr(expr_stmt) => {
            check_expr_for_sinks(
                &expr_stmt.value,
                analyzer,
                state,
                findings,
                file_path,
                line_index,
            );
        }
        Stmt::Return(ret) => {
            if let Some(value) = &ret.value {
                check_expr_for_sinks(value, analyzer, state, findings, file_path, line_index);
            }
        }
        Stmt::If(if_stmt) => handle_if(if_stmt, analyzer, state, findings, file_path, line_index),
        Stmt::For(for_stmt) => {
            handle_for(for_stmt, analyzer, state, findings, file_path, line_index);
        }
        Stmt::While(while_stmt) => {
            handle_while(while_stmt, analyzer, state, findings, file_path, line_index);
        }
        Stmt::With(with_stmt) => {
            for s in &with_stmt.body {
                analyze_stmt(s, analyzer, state, findings, file_path, line_index);
            }
        }
        Stmt::Try(try_stmt) => {
            handle_try(try_stmt, analyzer, state, findings, file_path, line_index);
        }
        Stmt::FunctionDef(nested_func) => {
            handle_function_def(
                nested_func,
                analyzer,
                state,
                findings,
                file_path,
                line_index,
            );
        }
        _ => {}
    }
}

fn handle_assign(
    assign: &ast::StmtAssign,
    analyzer: &TaintAnalyzer,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
    line_index: &LineIndex,
) {
    check_expr_for_sinks(
        &assign.value,
        analyzer,
        state,
        findings,
        file_path,
        line_index,
    );

    if let Some(taint_info) = analyzer.plugins.check_sources(&assign.value, line_index) {
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
    analyzer: &TaintAnalyzer,
    state: &mut TaintState,
    _findings: &mut Vec<TaintFinding>,
    _file_path: &Path,
    line_index: &LineIndex,
) {
    if let Some(value) = &assign.value {
        if let Some(taint_info) = analyzer.plugins.check_sources(value, line_index) {
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
    analyzer: &TaintAnalyzer,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
    line_index: &LineIndex,
) {
    if let Some(taint_info) = is_expr_tainted(&assign.value, state) {
        if let Expr::Name(name) = &*assign.target {
            state.mark_tainted(name.id.as_str(), taint_info.extend_path(name.id.as_str()));
        }
    }
    check_expr_for_sinks(
        &assign.value,
        analyzer,
        state,
        findings,
        file_path,
        line_index,
    );
}

fn handle_if(
    if_stmt: &ast::StmtIf,
    analyzer: &TaintAnalyzer,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
    line_index: &LineIndex,
) {
    check_expr_for_sinks(
        &if_stmt.test,
        analyzer,
        state,
        findings,
        file_path,
        line_index,
    );

    let mut then_state = state.clone();

    for s in &if_stmt.body {
        analyze_stmt(
            s,
            analyzer,
            &mut then_state,
            findings,
            file_path,
            line_index,
        );
    }

    let mut combined_state = then_state;

    for clause in &if_stmt.elif_else_clauses {
        let mut clause_state = state.clone();
        if let Some(test) = &clause.test {
            check_expr_for_sinks(test, analyzer, state, findings, file_path, line_index);
        }
        for s in &clause.body {
            analyze_stmt(
                s,
                analyzer,
                &mut clause_state,
                findings,
                file_path,
                line_index,
            );
        }
        combined_state.merge(&clause_state);
    }

    *state = combined_state;
}

fn handle_for(
    for_stmt: &ast::StmtFor,
    analyzer: &TaintAnalyzer,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
    line_index: &LineIndex,
) {
    if let Some(taint_info) = is_expr_tainted(&for_stmt.iter, state) {
        if let Expr::Name(name) = &*for_stmt.target {
            state.mark_tainted(name.id.as_str(), taint_info);
        }
    }

    for s in &for_stmt.body {
        analyze_stmt(s, analyzer, state, findings, file_path, line_index);
    }
    for s in &for_stmt.orelse {
        analyze_stmt(s, analyzer, state, findings, file_path, line_index);
    }
}

fn handle_while(
    while_stmt: &ast::StmtWhile,
    analyzer: &TaintAnalyzer,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
    line_index: &LineIndex,
) {
    check_expr_for_sinks(
        &while_stmt.test,
        analyzer,
        state,
        findings,
        file_path,
        line_index,
    );

    for s in &while_stmt.body {
        analyze_stmt(s, analyzer, state, findings, file_path, line_index);
    }
    for s in &while_stmt.orelse {
        analyze_stmt(s, analyzer, state, findings, file_path, line_index);
    }
}

fn handle_try(
    try_stmt: &ast::StmtTry,
    analyzer: &TaintAnalyzer,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
    line_index: &LineIndex,
) {
    for s in &try_stmt.body {
        analyze_stmt(s, analyzer, state, findings, file_path, line_index);
    }
    for handler in &try_stmt.handlers {
        let ast::ExceptHandler::ExceptHandler(h) = handler;
        for s in &h.body {
            analyze_stmt(s, analyzer, state, findings, file_path, line_index);
        }
    }
    for s in &try_stmt.orelse {
        analyze_stmt(s, analyzer, state, findings, file_path, line_index);
    }
    for s in &try_stmt.finalbody {
        analyze_stmt(s, analyzer, state, findings, file_path, line_index);
    }
}

fn handle_function_def(
    func: &ast::StmtFunctionDef,
    analyzer: &TaintAnalyzer,
    _state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
    line_index: &LineIndex,
) {
    let mut func_findings = analyze_function(func, analyzer, file_path, line_index, None);
    findings.append(&mut func_findings);
}

fn handle_call_sink(
    call: &ast::ExprCall,
    analyzer: &TaintAnalyzer,
    state: &TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
    line_index: &LineIndex,
) {
    // Check if this call is a sink
    if let Some(sink_info) = analyzer.plugins.check_sinks(call) {
        // Check if any dangerous argument is tainted
        if sink_info.dangerous_args.is_empty() {
            // Sentinel: check all positional arguments
            for arg in &call.arguments.args {
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
                        line_index.line_index(call.range().start()),
                        file_path,
                    );
                    findings.push(finding);
                }
            }
        } else {
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
                            line_index.line_index(call.range().start()),
                            file_path,
                        );
                        findings.push(finding);
                    }
                }
            }
        }

        // Check if receiver is tainted for method calls
        if let Expr::Attribute(attr) = &*call.func {
            if let Some(taint_info) = is_expr_tainted(&attr.value, state) {
                let finding = create_finding(
                    &taint_info,
                    &sink_info,
                    line_index.line_index(call.range().start()),
                    file_path,
                );
                findings.push(finding);
            }
        }

        // Check if any dangerous keyword is tainted
        for keyword in &call.arguments.keywords {
            if let Some(arg_name) = &keyword.arg {
                if sink_info.dangerous_keywords.contains(&arg_name.to_string()) {
                    if let Some(taint_info) = is_expr_tainted(&keyword.value, state) {
                        let finding = create_finding(
                            &taint_info,
                            &sink_info,
                            line_index.line_index(call.range().start()),
                            file_path,
                        );
                        findings.push(finding);
                    }
                }
            }
        }
    }

    // Recursively check arguments
    for arg in &call.arguments.args {
        check_expr_for_sinks(arg, analyzer, state, findings, file_path, line_index);
    }

    // Recursively check keyword arguments
    for keyword in &call.arguments.keywords {
        check_expr_for_sinks(
            &keyword.value,
            analyzer,
            state,
            findings,
            file_path,
            line_index,
        );
    }
}

/// Checks an expression for dangerous sink calls.
fn check_expr_for_sinks(
    expr: &Expr,
    analyzer: &TaintAnalyzer,
    state: &TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
    line_index: &LineIndex,
) {
    match expr {
        Expr::Call(call) => {
            handle_call_sink(call, analyzer, state, findings, file_path, line_index);
        }

        Expr::BinOp(binop) => {
            check_expr_for_sinks(
                &binop.left,
                analyzer,
                state,
                findings,
                file_path,
                line_index,
            );
            check_expr_for_sinks(
                &binop.right,
                analyzer,
                state,
                findings,
                file_path,
                line_index,
            );
        }

        Expr::If(ifexp) => {
            check_expr_for_sinks(
                &ifexp.test,
                analyzer,
                state,
                findings,
                file_path,
                line_index,
            );
            check_expr_for_sinks(
                &ifexp.body,
                analyzer,
                state,
                findings,
                file_path,
                line_index,
            );
            check_expr_for_sinks(
                &ifexp.orelse,
                analyzer,
                state,
                findings,
                file_path,
                line_index,
            );
        }

        Expr::List(list) => {
            for elt in &list.elts {
                check_expr_for_sinks(elt, analyzer, state, findings, file_path, line_index);
            }
        }

        Expr::ListComp(comp) => {
            check_expr_for_sinks(&comp.elt, analyzer, state, findings, file_path, line_index);
        }

        _ => {}
    }
}

/// Creates a taint finding from source and sink info.
fn create_finding(
    taint_info: &TaintInfo,
    sink_info: &SinkMatch,
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
