//! Data types for TODO/FIXME/HACK/XXX placeholder and debug-print detection.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Category of detected code annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoKind {
    /// `TODO` annotation — planned work not yet done.
    Todo,
    /// `FIXME` annotation — known defect that must be addressed.
    Fixme,
    /// `HACK` annotation — workaround or non-ideal implementation.
    Hack,
    /// `XXX` annotation — danger marker or code requiring attention.
    Xxx,
    /// Debug `print` / `console.log` / `println!` left in non-output code.
    DebugPrint,
    /// Commented-out code block (e.g. `# if x:`, `// for ...`).
    CommentedCode,
}

impl TodoKind {
    /// Short human-readable label used in terminal output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "TODO",
            Self::Fixme => "FIXME",
            Self::Hack => "HACK",
            Self::Xxx => "XXX",
            Self::DebugPrint => "debug_print",
            Self::CommentedCode => "commented_code",
        }
    }

    /// Single-line description used in terminal output and JSON.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::Todo => "TODO placeholder left in code",
            Self::Fixme => "FIXME placeholder left in code",
            Self::Hack => "HACK placeholder left in code",
            Self::Xxx => "XXX placeholder left in code",
            Self::DebugPrint => "Debug print statement left in non-output code",
            Self::CommentedCode => "Commented-out code block",
        }
    }
}

impl std::fmt::Display for TodoKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single detected annotation match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoMatch {
    /// Source file.
    pub file: PathBuf,
    /// 1-indexed source line number.
    pub line: usize,
    /// Category of annotation.
    pub kind: TodoKind,
    /// Human-readable description of the annotation.
    pub description: String,
    /// Trimmed source line text (capped at 120 chars).
    pub snippet: String,
}

/// Aggregate counts across all detected annotation kinds.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoStats {
    /// Total number of detected annotations.
    pub total: usize,
    /// Count of `TODO` annotations.
    pub todo_count: usize,
    /// Count of `FIXME` annotations.
    pub fixme_count: usize,
    /// Count of `HACK` annotations.
    pub hack_count: usize,
    /// Count of `XXX` annotations.
    pub xxx_count: usize,
    /// Count of debug print statements.
    pub debug_print_count: usize,
    /// Count of commented-out code blocks.
    pub commented_code_count: usize,
    /// Number of distinct source files that contain at least one annotation.
    pub affected_files: usize,
}

impl TodoStats {
    /// Increments the counter for the specified kind and the running total.
    pub fn increment(&mut self, kind: TodoKind) {
        self.total += 1;
        match kind {
            TodoKind::Todo => self.todo_count += 1,
            TodoKind::Fixme => self.fixme_count += 1,
            TodoKind::Hack => self.hack_count += 1,
            TodoKind::Xxx => self.xxx_count += 1,
            TodoKind::DebugPrint => self.debug_print_count += 1,
            TodoKind::CommentedCode => self.commented_code_count += 1,
        }
    }
}

/// Complete result of TODO/annotation scan across a set of source files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodosResult {
    /// Aggregate annotation counts.
    pub stats: TodoStats,
    /// All detected annotation matches, sorted by file then line.
    pub matches: Vec<TodoMatch>,
}

impl TodosResult {
    /// Returns `true` when no annotations were detected.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.stats.total == 0
    }
}
