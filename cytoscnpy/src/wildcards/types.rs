//! Data types for wildcard-import (`from module import *`) detection.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single detected wildcard import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WildcardMatch {
    /// Source file containing the wildcard import.
    pub file: PathBuf,
    /// 1-indexed line number of the `from … import *` statement.
    pub line: usize,
    /// The module being star-imported (e.g. `"os.path"`, `"typing"`).
    pub module: String,
    /// Trimmed source snippet at the match line (capped at 120 chars).
    pub snippet: String,
}

/// Aggregate counts for wildcard-import scan results.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WildcardStats {
    /// Total wildcard imports detected.
    pub total: usize,
    /// Number of distinct source files containing at least one wildcard import.
    pub affected_files: usize,
}

impl WildcardStats {
    /// Increments the running total.
    pub fn increment(&mut self) {
        self.total += 1;
    }
}

/// Complete result of a wildcard-import scan across a set of source files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WildcardsResult {
    /// Aggregate counts.
    pub stats: WildcardStats,
    /// All detected matches, sorted by file then line.
    pub matches: Vec<WildcardMatch>,
}

impl WildcardsResult {
    /// Returns `true` when no wildcard imports were detected.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.stats.total == 0
    }
}
