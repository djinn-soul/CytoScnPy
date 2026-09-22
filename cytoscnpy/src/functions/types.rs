//! Types and models for function metrics extraction and aggregation.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Structured record of an extracted function or method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionInfo {
    /// Qualified function name (e.g. `ClassName.method_name` or `function_name`).
    pub name: String,
    /// Unqualified identifier name (e.g. `method_name`).
    pub simple_name: String,
    /// Source file path where the function is defined.
    pub file: PathBuf,
    /// 1-based start line of the function definition (including decorators).
    pub start_line: usize,
    /// 1-based end line of the function definition.
    pub end_line: usize,
    /// Physical line count spanned (`end_line - start_line + 1`).
    pub line_count: usize,
    /// `McCabe` cyclomatic complexity (starts at 1).
    pub cyclomatic_complexity: usize,
    /// Maximum control flow nesting depth inside the function body.
    pub max_nesting: usize,
    /// Whether this is an `async def` function.
    pub is_async: bool,
    /// Whether this function is defined within a class.
    pub is_method: bool,
    /// Enclosing class name if a method.
    pub class_name: Option<String>,
}

/// Aggregate metrics across all extracted functions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionStats {
    /// Total number of functions extracted.
    pub total_functions: usize,
    /// Average cyclomatic complexity across all functions.
    pub avg_complexity: f64,
    /// Maximum cyclomatic complexity observed.
    pub max_complexity: usize,
    /// Average line count across all functions.
    pub avg_lines: f64,
    /// Maximum line count observed.
    pub max_lines: usize,
    /// Average max nesting depth across all functions.
    pub avg_nesting: f64,
    /// Maximum nesting depth observed.
    pub max_nesting: usize,
}

impl FunctionStats {
    /// Compute aggregate statistics from a slice of `FunctionInfo`.
    #[must_use]
    pub fn from_functions(functions: &[FunctionInfo]) -> Self {
        if functions.is_empty() {
            return Self {
                total_functions: 0,
                avg_complexity: 0.0,
                max_complexity: 0,
                avg_lines: 0.0,
                max_lines: 0,
                avg_nesting: 0.0,
                max_nesting: 0,
            };
        }
        let total = functions.len();
        let total_complexity: usize = functions.iter().map(|f| f.cyclomatic_complexity).sum();
        let max_complexity = functions
            .iter()
            .map(|f| f.cyclomatic_complexity)
            .max()
            .unwrap_or(0);
        let total_lines: usize = functions.iter().map(|f| f.line_count).sum();
        let max_lines = functions.iter().map(|f| f.line_count).max().unwrap_or(0);
        let total_nesting: usize = functions.iter().map(|f| f.max_nesting).sum();
        let max_nesting = functions.iter().map(|f| f.max_nesting).max().unwrap_or(0);

        Self {
            total_functions: total,
            avg_complexity: total_complexity as f64 / total as f64,
            max_complexity,
            avg_lines: total_lines as f64 / total as f64,
            max_lines,
            avg_nesting: total_nesting as f64 / total as f64,
            max_nesting,
        }
    }
}

/// Complete result of function extraction across a scanned codebase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionsResult {
    /// All extracted functions.
    pub functions: Vec<FunctionInfo>,
    /// Aggregate function metrics.
    pub stats: FunctionStats,
    /// Total Python files scanned.
    pub files_scanned: usize,
}

impl FunctionsResult {
    /// Check whether no functions were found or if all functions have zero/minimal issues.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }
}
