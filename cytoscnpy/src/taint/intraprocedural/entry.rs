use super::handlers::{
    handle_ann_assign, handle_assign, handle_aug_assign, handle_for, handle_function_def,
    handle_if, handle_try, handle_while,
};
use super::sinks::check_expr_for_sinks;
use crate::taint::analyzer::TaintAnalyzer;
use crate::taint::propagation::TaintState;
use crate::taint::types::{TaintFinding, TaintInfo, TaintSource};
use crate::utils::LineIndex;
use ruff_python_ast::{self as ast, Stmt};
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

    // Always taint function parameters (conservative approach).
    for arg in &func.parameters.args {
        let name = arg.parameter.name.as_str();
        state.mark_tainted(
            name,
            TaintInfo::new(
                TaintSource::FunctionParam(name.to_owned()),
                line_index.line_index(arg.range().start()),
            ),
        );
    }

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

    // Always taint function parameters (conservative approach).
    for arg in &func.parameters.args {
        let name = arg.parameter.name.as_str();
        state.mark_tainted(
            name,
            TaintInfo::new(
                TaintSource::FunctionParam(name.to_owned()),
                line_index.line_index(arg.range().start()),
            ),
        );
    }

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
pub(super) fn analyze_stmt(
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
            handle_ann_assign(assign, analyzer, state, line_index);
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
            for nested in &with_stmt.body {
                analyze_stmt(nested, analyzer, state, findings, file_path, line_index);
            }
        }
        Stmt::Try(try_stmt) => {
            handle_try(try_stmt, analyzer, state, findings, file_path, line_index);
        }
        Stmt::FunctionDef(nested_func) => {
            handle_function_def(nested_func, analyzer, findings, file_path, line_index);
        }
        _ => {}
    }
}
