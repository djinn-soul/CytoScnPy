//! Data models for unreferenced large functions in isolated files.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single unreferenced large function detected in an isolated file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnreferencedFunction {
    /// File containing the unreferenced function.
    pub file: PathBuf,
    /// Function name.
    pub name: String,
    /// 1-indexed starting line.
    pub start_line: usize,
    /// 1-indexed ending line (inclusive).
    pub end_line: usize,
    /// Physical lines spanned by this function.
    pub line_count: usize,
    /// Function kind (e.g. "function", "method").
    pub node_kind: String,
    /// Name of enclosing class if this is a method.
    pub class_name: Option<String>,
}

/// Summary of an isolated file (0 incoming imports) containing unreferenced functions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsolatedFileSummary {
    /// File path.
    pub file: PathBuf,
    /// Total physical lines in file.
    pub total_lines: usize,
    /// Count of unreferenced functions in this file.
    pub unreferenced_functions: usize,
    /// Total lines spanned by unreferenced functions.
    pub unreferenced_lines: usize,
}

/// Aggregate statistics for unreferenced function analysis.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnreferencedStats {
    /// Total number of unreferenced large functions found.
    pub total_unreferenced_functions: usize,
    /// Total lines in unreferenced large functions.
    pub total_unreferenced_lines: usize,
    /// Number of isolated files (0 incoming imports) with unreferenced functions.
    pub isolated_files_count: usize,
    /// Total Python files scanned.
    pub total_scanned_files: usize,
    /// Total physical lines scanned across all files.
    pub total_scanned_lines: usize,
}

/// Overall results of unreferenced function analysis.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnreferencedResult {
    /// Detected unreferenced functions.
    pub items: Vec<UnreferencedFunction>,
    /// Summaries for isolated files containing unreferenced functions.
    pub isolated_files: Vec<IsolatedFileSummary>,
    /// Summary statistics.
    pub stats: UnreferencedStats,
    /// Root paths analyzed.
    pub roots: Vec<PathBuf>,
}

impl UnreferencedResult {
    /// Returns `true` if no unreferenced large functions were found.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.items.is_empty()
    }
}
