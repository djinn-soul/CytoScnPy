use crate::taint::analyzer::TaintAnalyzer;
use crate::taint::propagation::{is_expr_tainted, is_parameterized_query, TaintState};
use crate::taint::types::{score_exploitability, SinkMatch, TaintFinding, TaintInfo, VulnType};
use crate::utils::LineIndex;
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;
use std::path::Path;

fn handle_call_sink(
    call: &ast::ExprCall,
    analyzer: &TaintAnalyzer,
    state: &TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
    line_index: &LineIndex,
) {
    if let Some(sink_info) = analyzer.plugins.check_sinks(call) {
        if sink_info.dangerous_args.is_empty() {
            for arg in &call.arguments.args {
                if let Some(taint_info) = is_expr_tainted(arg, state) {
                    if sink_info.vuln_type == VulnType::SqlInjection && is_parameterized_query(call)
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
                        if sink_info.vuln_type == VulnType::SqlInjection
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

    for arg in &call.arguments.args {
        check_expr_for_sinks(arg, analyzer, state, findings, file_path, line_index);
    }

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
pub(super) fn check_expr_for_sinks(
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
            for element in &list.elts {
                check_expr_for_sinks(element, analyzer, state, findings, file_path, line_index);
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
        category: "Taint Analysis".to_owned(),
        sink: sink_info.name.clone(),
        rule_id: sink_info.rule_id.clone(),
        sink_line,
        sink_col: 0,
        flow_path: taint_info.path.clone(),
        vuln_type: sink_info.vuln_type.clone(),
        severity: sink_info.severity,
        file: file_path.to_path_buf(),
        remediation: sink_info.remediation.clone(),
        exploitability_score: score_exploitability(
            &taint_info.source,
            &sink_info.vuln_type,
            sink_info.severity,
            taint_info.path.len(),
        ),
    }
}
