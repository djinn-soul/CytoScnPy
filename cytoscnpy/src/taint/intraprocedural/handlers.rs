use super::entry::{analyze_function, analyze_stmt};
use super::sinks::check_expr_for_sinks;
use crate::taint::analyzer::TaintAnalyzer;
use crate::taint::propagation::{is_expr_tainted, TaintState};
use crate::taint::sanitizers::{call_argument_names, SanitizerKind};
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
    apply_side_effect_sanitizer(&assign.value, analyzer, state);

    if let Some(taint_info) = analyzer.plugins.check_sources(&assign.value, line_index) {
        for target in &assign.targets {
            if let Expr::Name(name) = target {
                state.mark_tainted(name.id.as_str(), taint_info.clone());
            }
        }
    } else if let Some(taint_info) = is_expr_tainted(&assign.value, state) {
        if let Expr::Call(call) = &*assign.value {
            let sanitized_types = analyzer.sanitizer_types(call, SanitizerKind::ReturnValue);
            if !sanitized_types.is_empty() {
                for target in &assign.targets {
                    if let Expr::Name(name) = target {
                        state.mark_tainted(
                            name.id.as_str(),
                            taint_info
                                .extend_path(name.id.as_str())
                                .with_sanitized_for(&sanitized_types),
                        );
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
        apply_side_effect_sanitizer(value, analyzer, state);
        if let Some(taint_info) = analyzer.plugins.check_sources(value, line_index) {
            if let Expr::Name(name) = &*assign.target {
                state.mark_tainted(name.id.as_str(), taint_info);
            }
        } else if let Some(taint_info) = is_expr_tainted(value, state) {
            if let Expr::Call(call) = &**value {
                let sanitized_types = analyzer.sanitizer_types(call, SanitizerKind::ReturnValue);
                if !sanitized_types.is_empty() {
                    if let Expr::Name(name) = &*assign.target {
                        state.mark_tainted(
                            name.id.as_str(),
                            taint_info
                                .extend_path(name.id.as_str())
                                .with_sanitized_for(&sanitized_types),
                        );
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
    apply_side_effect_sanitizer(&assign.value, analyzer, state);
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
    apply_guard_sanitizer(&if_stmt.test, analyzer, &mut then_state);
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
    let mut has_else = false;
    for clause in &if_stmt.elif_else_clauses {
        let mut clause_state = state.clone();
        if let Some(test) = &clause.test {
            check_expr_for_sinks(test, analyzer, state, findings, file_path, line_index);
            apply_guard_sanitizer(test, analyzer, &mut clause_state);
        } else {
            has_else = true;
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
    if !has_else {
        combined_state.merge(state);
    }

    *state = combined_state;
}

pub(super) fn apply_side_effect_sanitizer(
    expr: &Expr,
    analyzer: &TaintAnalyzer,
    state: &mut TaintState,
) {
    let Expr::Call(call) = expr else {
        return;
    };
    let sanitized_types = analyzer.sanitizer_types(call, SanitizerKind::SideEffect);
    sanitize_call_arguments(call, state, &sanitized_types);
}

fn apply_guard_sanitizer(expr: &Expr, analyzer: &TaintAnalyzer, state: &mut TaintState) {
    match expr {
        Expr::Call(call) => {
            let sanitized_types = analyzer.sanitizer_types(call, SanitizerKind::Guard);
            sanitize_call_arguments(call, state, &sanitized_types);
        }
        Expr::BoolOp(bool_op) => {
            for value in &bool_op.values {
                apply_guard_sanitizer(value, analyzer, state);
            }
        }
        _ => {}
    }
}

fn sanitize_call_arguments(
    call: &ast::ExprCall,
    state: &mut TaintState,
    sanitized_types: &[crate::taint::types::VulnType],
) {
    if sanitized_types.is_empty() {
        return;
    }
    for name in call_argument_names(call) {
        state.sanitize_for(&name, sanitized_types);
    }
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
