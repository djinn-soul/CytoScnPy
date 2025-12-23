//! Function summaries for interprocedural analysis.
//!
//! Caches taint behavior of functions to avoid re-analysis.

use super::intraprocedural;
use super::propagation::TaintState;
use super::types::{FunctionSummary, TaintSource};
use ruff_python_ast::{self as ast, Stmt};
use std::collections::HashMap;
use std::path::Path;

/// Database of function summaries.
#[derive(Debug, Default)]
pub struct SummaryDatabase {
    /// Map from qualified function name to summary
    pub summaries: HashMap<String, FunctionSummary>,
}

impl SummaryDatabase {
    /// Creates a new empty database.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets or computes the summary for a function.
    pub fn get_or_compute(
        &mut self,
        func: &ast::StmtFunctionDef,
        file_path: &Path,
    ) -> FunctionSummary {
        let name = func.name.to_string();

        if let Some(summary) = self.summaries.get(&name) {
            return summary.clone();
        }

        let summary = compute_summary(func, file_path);
        self.summaries.insert(name, summary.clone());
        summary
    }

    /// Gets the summary for a function by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&FunctionSummary> {
        self.summaries.get(name)
    }

    /// Checks if a function taints its return value.
    #[must_use]
    pub fn function_taints_return(&self, name: &str) -> bool {
        self.summaries.get(name).is_some_and(|s| s.returns_tainted)
    }

    /// Checks which parameters of a function propagate to return.
    #[must_use]
    pub fn get_param_to_return(&self, name: &str) -> Vec<usize> {
        self.summaries
            .get(name)
            .map(|s| {
                s.param_to_return
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &taints)| if taints { Some(i) } else { None })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Computes the summary for a function.
fn compute_summary(func: &ast::StmtFunctionDef, file_path: &Path) -> FunctionSummary {
    let param_count = func.parameters.args.len();
    let mut summary = FunctionSummary::new(&func.name, param_count);

    // Create initial taint state with parameters marked as tainted
    let mut param_taint_states: Vec<TaintState> = Vec::new();
    let mut param_indices: Vec<usize> = Vec::new(); // Track original param indices

    for (i, arg) in func.parameters.args.iter().enumerate() {
        let mut state = TaintState::new();
        let param_name = arg.parameter.name.to_string();

        // Skip self/cls
        if param_name == "self" || param_name == "cls" {
            continue;
        }

        state.mark_tainted(
            &param_name,
            super::types::TaintInfo::new(
                TaintSource::FunctionParam(param_name.clone()),
                func.range.start().to_u32() as usize,
            ),
        );
        param_taint_states.push(state);
        param_indices.push(i);
    }

    // Analyze function with each param tainted
    for (state_idx, state) in param_taint_states.iter().enumerate() {
        let original_param_idx = param_indices[state_idx];

        // Analyze function with this param tainted
        let findings = intraprocedural::analyze_function(func, file_path, Some(state.clone()));

        // Record sinks reached
        for finding in findings {
            summary
                .param_to_sinks
                .push((original_param_idx, finding.vuln_type));
            summary.has_sinks = true;
        }
    }

    // Check if function returns tainted data
    for stmt in &func.body {
        if let Stmt::Return(ret) = stmt {
            if let Some(value) = &ret.value {
                // Check if return value contains any taint sources
                if contains_taint_source(value) {
                    summary.returns_tainted = true;
                }
            }
        }
    }

    summary
}

/// Checks if an expression contains a taint source.
fn contains_taint_source(expr: &ast::Expr) -> bool {
    super::sources::check_taint_source(expr).is_some()
}

/// Prebuilt summaries for common library functions.
#[must_use]
pub fn get_builtin_summaries() -> HashMap<String, FunctionSummary> {
    let mut summaries = HashMap::new();

    // input() returns tainted data
    let mut input_summary = FunctionSummary::new("input", 0);
    input_summary.returns_tainted = true;
    summaries.insert("input".to_owned(), input_summary);

    // os.getenv() returns tainted data
    let mut getenv_summary = FunctionSummary::new("os.getenv", 1);
    getenv_summary.returns_tainted = true;
    summaries.insert("os.getenv".to_owned(), getenv_summary);

    // int() sanitizes
    let int_summary = FunctionSummary::new("int", 1);
    summaries.insert("int".to_owned(), int_summary);

    // float() sanitizes
    let float_summary = FunctionSummary::new("float", 1);
    summaries.insert("float".to_owned(), float_summary);

    summaries
}
