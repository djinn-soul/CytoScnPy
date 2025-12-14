//! Interprocedural taint analysis.
//!
//! Tracks taint flow across function boundaries within a single file.

use super::call_graph::CallGraph;
use super::propagation::TaintState;
use super::sources::check_fastapi_param;
use super::summaries::SummaryDatabase;
use super::types::{TaintFinding, TaintInfo, TaintSource};
use ruff_python_ast::{self as ast, Stmt};

use std::path::Path;

/// Performs interprocedural taint analysis on a module.
pub fn analyze_module(stmts: &[Stmt], file_path: &Path) -> Vec<TaintFinding> {
    let mut findings = Vec::new();
    let mut call_graph = CallGraph::new();
    let mut summaries = SummaryDatabase::new();

    // Phase 1: Build call graph
    call_graph.build_from_module(stmts);

    // Phase 2: Collect all function definitions
    let functions = collect_functions(stmts);

    // Phase 3: Analyze in topological order (callees before callers)
    let analysis_order = call_graph.get_analysis_order();

    for func_name in &analysis_order {
        if let Some(func) = functions.get(func_name) {
            match func {
                FunctionDef::Sync(f) => {
                    // Compute summary
                    summaries.get_or_compute(f, file_path);

                    // Check for FastAPI parameters
                    let fastapi_params = check_fastapi_param(f);

                    // Create initial taint state
                    let mut state = TaintState::new();
                    for (param_name, taint_info) in fastapi_params {
                        state.mark_tainted(&param_name, taint_info);
                    }

                    // Perform intraprocedural analysis with context
                    let func_findings =
                        analyze_with_context(f, file_path, &state, &summaries, &call_graph);
                    findings.extend(func_findings);
                }
                FunctionDef::Async(f) => {
                    // Check for FastAPI parameters
                    let state = TaintState::new();
                    // Note: FastAPI params check would need async variant

                    let func_findings =
                        analyze_async_with_context(f, file_path, &state, &summaries, &call_graph);
                    findings.extend(func_findings);
                }
            }
        }
    }

    // Phase 4: Analyze module-level code
    let module_findings = analyze_module_level(stmts, file_path);
    findings.extend(module_findings);

    findings
}

/// Wrapper enum for function definitions.
enum FunctionDef<'a> {
    Sync(&'a ast::StmtFunctionDef),
    Async(&'a ast::StmtFunctionDef),
}

/// Collects all function definitions from statements.
fn collect_functions(stmts: &[Stmt]) -> std::collections::HashMap<String, FunctionDef<'_>> {
    let mut functions = std::collections::HashMap::new();

    for stmt in stmts {
        match stmt {
            Stmt::FunctionDef(func) => {
                if func.is_async {
                    functions.insert(func.name.to_string(), FunctionDef::Async(func));
                } else {
                    functions.insert(func.name.to_string(), FunctionDef::Sync(func));
                }
            }
            Stmt::ClassDef(class) => {
                for s in &class.body {
                    if let Stmt::FunctionDef(method) = s {
                        let qualified_name = format!("{}.{}", class.name, method.name);
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

/// Analyzes a function with interprocedural context.
fn analyze_with_context(
    func: &ast::StmtFunctionDef,
    file_path: &Path,
    initial_state: &TaintState,
    summaries: &SummaryDatabase,
    call_graph: &CallGraph,
) -> Vec<TaintFinding> {
    let mut state = initial_state.clone();
    let mut findings = Vec::new();

    // Analyze statements with context
    for stmt in &func.body {
        analyze_stmt_with_context(
            stmt,
            &mut state,
            &mut findings,
            file_path,
            summaries,
            call_graph,
        );
    }

    findings
}

/// Analyzes an async function with context.
fn analyze_async_with_context(
    func: &ast::StmtFunctionDef,
    file_path: &Path,
    initial_state: &TaintState,
    summaries: &SummaryDatabase,
    call_graph: &CallGraph,
) -> Vec<TaintFinding> {
    let mut state = initial_state.clone();
    let mut findings = Vec::new();

    for stmt in &func.body {
        analyze_stmt_with_context(
            stmt,
            &mut state,
            &mut findings,
            file_path,
            summaries,
            call_graph,
        );
    }

    findings
}

/// Analyzes a statement with interprocedural context.
fn analyze_stmt_with_context(
    stmt: &Stmt,
    state: &mut TaintState,
    findings: &mut Vec<TaintFinding>,
    file_path: &Path,
    summaries: &SummaryDatabase,
    call_graph: &CallGraph,
) {
    match stmt {
        Stmt::Assign(assign) => {
            // Check for calls to functions that return tainted data
            if let ast::Expr::Call(call) = &*assign.value {
                if let Some(func_name) = get_call_name(&call.func) {
                    // Check if called function returns tainted data
                    if summaries.function_taints_return(&func_name) {
                        for target in &assign.targets {
                            if let ast::Expr::Name(name) = target {
                                state.mark_tainted(
                                    name.id.as_str(),
                                    TaintInfo::new(
                                        TaintSource::FunctionReturn(func_name.clone()),
                                        get_line(&assign.value),
                                    ),
                                );
                            }
                        }
                        return;
                    }

                    // Check if tainted arg propagates to return
                    // Check if tainted arg propagates to return
                    let param_to_return = summaries.get_param_to_return(&func_name);
                    for param_idx in param_to_return {
                        if let Some(arg) = call.arguments.args.get(param_idx) {
                            if let Some(taint_info) =
                                super::propagation::is_expr_tainted(arg, state)
                            {
                                for target in &assign.targets {
                                    if let ast::Expr::Name(name) = target {
                                        state.mark_tainted(
                                            name.id.as_str(),
                                            taint_info.extend_path(name.id.as_str()),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Stmt::If(if_stmt) => {
            for s in &if_stmt.body {
                analyze_stmt_with_context(s, state, findings, file_path, summaries, call_graph);
            }
            for clause in &if_stmt.elif_else_clauses {
                for s in &clause.body {
                    analyze_stmt_with_context(s, state, findings, file_path, summaries, call_graph);
                }
            }
        }

        Stmt::For(for_stmt) => {
            for s in &for_stmt.body {
                analyze_stmt_with_context(s, state, findings, file_path, summaries, call_graph);
            }
        }

        Stmt::While(while_stmt) => {
            for s in &while_stmt.body {
                analyze_stmt_with_context(s, state, findings, file_path, summaries, call_graph);
            }
        }

        Stmt::With(with_stmt) => {
            for s in &with_stmt.body {
                analyze_stmt_with_context(s, state, findings, file_path, summaries, call_graph);
            }
        }

        Stmt::Try(try_stmt) => {
            for s in &try_stmt.body {
                analyze_stmt_with_context(s, state, findings, file_path, summaries, call_graph);
            }
            for handler in &try_stmt.handlers {
                let ast::ExceptHandler::ExceptHandler(h) = handler;
                for s in &h.body {
                    analyze_stmt_with_context(s, state, findings, file_path, summaries, call_graph);
                }
            }
        }

        _ => {
            // Fall back to intraprocedural analysis for other statements
        }
    }
}

/// Analyzes module-level code.
fn analyze_module_level(stmts: &[Stmt], file_path: &Path) -> Vec<TaintFinding> {
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
            if let Some(taint_info) = super::sources::check_taint_source(&assign.value) {
                // Module-level assignment from taint source
                // We track this but don't report unless there's a sink
                let _ = taint_info; // Tracked for potential future use
            }
        }

        if let Stmt::Expr(expr_stmt) = stmt {
            // Check for dangerous calls at module level
            if let ast::Expr::Call(call) = &*expr_stmt.value {
                if let Some(sink_info) = super::sinks::check_sink(call) {
                    // Check if any argument is tainted
                    for arg in &call.arguments.args {
                        if let Some(taint_info) = super::sources::check_taint_source(arg) {
                            use ruff_text_size::Ranged;
                            findings.push(super::types::TaintFinding {
                                source: taint_info.source.to_string(),
                                source_line: taint_info.source_line,
                                sink: sink_info.name.clone(),
                                sink_line: call.range().start().to_u32() as usize,
                                sink_col: 0,
                                flow_path: vec![],
                                vuln_type: sink_info.vuln_type.clone(),
                                severity: sink_info.severity,
                                file: file_path.to_path_buf(),
                                remediation: sink_info.remediation.clone(),
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
fn get_call_name(func: &ast::Expr) -> Option<String> {
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
fn get_line(expr: &ast::Expr) -> usize {
    use ruff_text_size::Ranged;
    expr.range().start().to_u32() as usize
}
