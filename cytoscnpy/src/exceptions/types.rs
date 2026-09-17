//! Data types for bare-except and empty exception-handler detection.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Category of detected exception-handling anti-pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExceptionKind {
    /// `except:` with no exception type — silently swallows every possible error.
    BareExcept,
    /// Handler body is only `pass`, `...`, or a bare string literal (swallows the error).
    EmptyHandler,
}

impl ExceptionKind {
    /// Short human-readable label used in terminal output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BareExcept => "bare_except",
            Self::EmptyHandler => "empty_handler",
        }
    }

    /// Single-line description used in terminal output and JSON.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::BareExcept => "Bare except block catches every exception silently",
            Self::EmptyHandler => "Empty exception handler swallows the error without action",
        }
    }
}

impl std::fmt::Display for ExceptionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single detected exception-handler anti-pattern match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExceptionMatch {
    /// Source file containing the handler.
    pub file: PathBuf,
    /// 1-indexed line number of the `except` clause.
    pub line: usize,
    /// Category of anti-pattern detected.
    pub kind: ExceptionKind,
    /// Human-readable description.
    pub description: String,
    /// Trimmed source snippet at the match line (capped at 120 chars).
    pub snippet: String,
}

/// Aggregate counts across all detected exception-handler anti-patterns.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExceptionStats {
    /// Total number of detected anti-patterns.
    pub total: usize,
    /// Count of bare `except:` blocks.
    pub bare_except_count: usize,
    /// Count of empty exception handlers.
    pub empty_handler_count: usize,
    /// Number of distinct source files containing at least one match.
    pub affected_files: usize,
}

impl ExceptionStats {
    /// Increments the counter for the specified kind and the running total.
    pub fn increment(&mut self, kind: ExceptionKind) {
        self.total += 1;
        match kind {
            ExceptionKind::BareExcept => self.bare_except_count += 1,
            ExceptionKind::EmptyHandler => self.empty_handler_count += 1,
        }
    }
}

/// Complete result of exception-handler anti-pattern scan across a set of source files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExceptionsResult {
    /// Aggregate counts.
    pub stats: ExceptionStats,
    /// All detected matches, sorted by file then line.
    pub matches: Vec<ExceptionMatch>,
}

impl ExceptionsResult {
    /// Returns `true` when no anti-patterns were detected.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.stats.total == 0
    }
}
