use super::entry::{analyze_function, analyze_stmt};
use super::sinks::check_expr_for_sinks;
use crate::taint::analyzer::TaintAnalyzer;
use crate::taint::propagation::{is_expr_tainted, is_sanitizer_call, TaintState};
use crate::taint::types::TaintFinding;
use crate::utils::LineIndex;
use ruff_python_ast::{self as ast, Expr};
use std::path::Path;

pub(super) fn handle_assign(
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

pub(super) fn handle_ann_assign(
    assign: &ast::StmtAnnAssign,
    analyzer: &TaintAnalyzer,
    state: &mut TaintState,
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

pub(super) fn handle_aug_assign(
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

pub(super) fn handle_if(
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
    for nested in &if_stmt.body {
        analyze_stmt(
            nested,
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
        for nested in &clause.body {
            analyze_stmt(
                nested,
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

pub(super) fn handle_for(
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

    for nested in &for_stmt.body {
        analyze_stmt(nested, analyzer, state, findings, file_path, line_index);
    }
    for nested in &for_stmt.orelse {
        analyze_stmt(nested, analyzer, state, findings, file_path, line_index);
    }
}

pub(super) fn handle_while(
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

    for nested in &while_stmt.body {
        analyze_stmt(nested, analyzer, state, findings, file_path, line_index);
    }
    for nested in &while_stmt.orelse {
        analyze_stmt(nested, analyzer, state, findings, file_path, line_index);
    }
}

pub(super) fn handle_try(
    try_stmt: &ast::StmtTry,
    analyzer: &TaintAnalyzer,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
    line_index: &LineIndex,
) {
    for nested in &try_stmt.body {
        analyze_stmt(nested, analyzer, state, findings, file_path, line_index);
    }
    for handler in &try_stmt.handlers {
        let ast::ExceptHandler::ExceptHandler(except_handler) = handler;
        for nested in &except_handler.body {
            analyze_stmt(nested, analyzer, state, findings, file_path, line_index);
        }
    }
    for nested in &try_stmt.orelse {
        analyze_stmt(nested, analyzer, state, findings, file_path, line_index);
    }
    for nested in &try_stmt.finalbody {
        analyze_stmt(nested, analyzer, state, findings, file_path, line_index);
    }
}

pub(super) fn handle_function_def(
    func: &ast::StmtFunctionDef,
    analyzer: &TaintAnalyzer,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
    line_index: &LineIndex,
) {
    let mut func_findings = analyze_function(func, analyzer, file_path, line_index, None);
    findings.append(&mut func_findings);
}
