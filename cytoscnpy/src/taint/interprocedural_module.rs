//! Function collection, module-level taint checks, and expression helpers.

use super::super::analyzer::TaintAnalyzer;
use super::super::types::TaintFinding;
use crate::utils::LineIndex;
use ruff_python_ast::{self as ast, Stmt};
use rustc_hash::FxHashMap;
use std::path::Path;

/// Wrapper enum for function definitions.
pub(super) enum FunctionDef<'a> {
    Sync(&'a ast::StmtFunctionDef),
    Async(&'a ast::StmtFunctionDef),
}

/// Collects all function definitions from statements.
pub(super) fn collect_functions<'a>(
    stmts: &'a [Stmt],
    module_name: &str,
) -> FxHashMap<String, FunctionDef<'a>> {
    let mut functions = FxHashMap::default();

    for stmt in stmts {
        match stmt {
            Stmt::FunctionDef(func) => {
                let qualified_name = qualify_name(module_name, None, func.name.as_str());
                if func.is_async {
                    functions.insert(qualified_name, FunctionDef::Async(func));
                } else {
                    functions.insert(qualified_name, FunctionDef::Sync(func));
                }
            }
            Stmt::ClassDef(class) => {
                for s in &class.body {
                    if let Stmt::FunctionDef(method) = s {
                        let qualified_name = qualify_name(
                            module_name,
                            Some(class.name.as_str()),
                            method.name.as_str(),
                        );
                        if method.is_async {
                            functions.insert(qualified_name, FunctionDef::Async(method));
                        } else {
                            functions.insert(qualified_name, FunctionDef::Sync(method));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    functions
}

pub(super) fn qualify_name(module_name: &str, class_name: Option<&str>, func_name: &str) -> String {
    let mut qualified = String::new();
    if !module_name.is_empty() {
        qualified.push_str(module_name);
        qualified.push('.');
    }
    if let Some(class_name) = class_name {
        qualified.push_str(class_name);
        qualified.push('.');
    }
    qualified.push_str(func_name);
    qualified
}

/// Analyzes module-level code.
pub(super) fn analyze_module_level(
    stmts: &[Stmt],
    analyzer: &TaintAnalyzer,
    file_path: &Path,
    line_index: &LineIndex,
) -> Vec<TaintFinding> {
    let mut findings = Vec::new();

    for stmt in stmts {
        // Skip function/class definitions - they're analyzed separately
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => continue,
            _ => {}
        }

        // For module-level statements, do basic taint checking
        // This is simplified compared to function-level analysis
        if let Stmt::Assign(assign) = stmt {
            if let Some(taint_info) =
                super::super::sources::check_taint_source(&assign.value, line_index)
            {
                // Module-level assignment from taint source
                // We track this but don't report unless there's a sink
                let _ = taint_info; // Tracked for potential future use
            }
        }

        if let Stmt::Expr(expr_stmt) = stmt {
            // Check for dangerous calls at module level
            if let ast::Expr::Call(call) = &*expr_stmt.value {
                if let Some(sink_match) = analyzer.plugins.check_sinks(call) {
                    // Check if any argument is tainted
                    for arg in &call.arguments.args {
                        if let Some(taint_info) =
                            super::super::sources::check_taint_source(arg, line_index)
                        {
                            use ruff_text_size::Ranged;
                            findings.push(super::super::types::TaintFinding {
                                source: taint_info.source.to_string(),
                                source_line: taint_info.source_line,
                                category: "Taint Analysis".to_owned(),
                                sink: sink_match.name.clone(),
                                rule_id: sink_match.rule_id.clone(),
                                sink_line: line_index.line_index(call.range().start()),
                                sink_col: 0,
                                flow_path: vec![],
                                vuln_type: sink_match.vuln_type.clone(),
                                severity: sink_match.severity,
                                file: file_path.to_path_buf(),
                                remediation: sink_match.remediation.clone(),
                                exploitability_score: super::super::types::score_exploitability(
                                    &taint_info.source,
                                    &sink_match.vuln_type,
                                    sink_match.severity,
                                    0,
                                ),
                            });
                        }
                    }
                }
            }
        }
    }

    findings
}

/// Gets the call name from an expression.
pub(super) fn get_call_name(func: &ast::Expr) -> Option<String> {
    match func {
        ast::Expr::Name(node) => Some(node.id.to_string()),
        ast::Expr::Attribute(node) => {
            if let ast::Expr::Name(value) = &*node.value {
                Some(format!("{}.{}", value.id, node.attr))
            } else {
                Some(node.attr.to_string())
            }
        }
        _ => None,
    }
}

/// Gets line number from an expression.
pub(super) fn get_line(expr: &ast::Expr, line_index: &LineIndex) -> usize {
    use ruff_text_size::Ranged;
    line_index.line_index(expr.range().start())
}
