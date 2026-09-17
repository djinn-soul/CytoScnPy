//! Data types for anti-pattern detection (magic numbers and deeply nested callbacks).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Classification of detected anti-pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AntiPatternKind {
    /// Magic number used in conditional logic or comparisons (not 0, 1, 2, -1, 100).
    MagicNumber,
    /// Deeply nested callback, closure, or control block (depth >= 4).
    DeeplyNestedCallback,
}

impl AntiPatternKind {
    /// Short identifier for the anti-pattern kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MagicNumber => "magic_number",
            Self::DeeplyNestedCallback => "deeply_nested_callback",
        }
    }

    /// Human-readable description.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::MagicNumber => "Magic number in logic (not 0, 1, 2)",
            Self::DeeplyNestedCallback => "Callback nesting exceeds readable depth",
        }
    }
}

impl std::fmt::Display for AntiPatternKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A detected instance of an anti-pattern in source code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AntiPatternMatch {
    /// File path where the anti-pattern was detected.
    pub file: PathBuf,
    /// 1-indexed line number.
    pub line: usize,
    /// 1-indexed column number.
    pub column: usize,
    /// Category of the anti-pattern.
    pub kind: AntiPatternKind,
    /// Identifier string (e.g. `magic_number`, `deeply_nested_callback`).
    pub pattern_name: String,
    /// Description of the anti-pattern.
    pub description: String,
    /// Relevant code snippet.
    pub snippet: String,
}

/// Aggregated statistics of detected anti-patterns.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AntiPatternStats {
    /// Total count of detected anti-patterns.
    pub total: usize,
    /// Count of magic number anti-patterns.
    pub magic_numbers: usize,
    /// Count of deeply nested callback anti-patterns.
    pub nested_callbacks: usize,
    /// Number of distinct files containing at least one anti-pattern.
    pub affected_files: usize,
}

impl AntiPatternStats {
    /// Record a single match into statistics.
    pub fn record(&mut self, kind: AntiPatternKind) {
        self.total += 1;
        match kind {
            AntiPatternKind::MagicNumber => self.magic_numbers += 1,
            AntiPatternKind::DeeplyNestedCallback => self.nested_callbacks += 1,
        }
    }
}

/// Overall results of anti-pattern analysis across scanned files.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AntiPatternsResult {
    /// All detected anti-pattern occurrences.
    pub matches: Vec<AntiPatternMatch>,
    /// Summary statistics.
    pub stats: AntiPatternStats,
    /// Total number of Python files analyzed.
    pub files_scanned: usize,
    /// Target root paths that were scanned.
    pub roots: Vec<PathBuf>,
}

impl AntiPatternsResult {
    /// Returns `true` if no anti-patterns were detected.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.matches.is_empty()
    }
}
