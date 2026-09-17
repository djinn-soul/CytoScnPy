//! Anti-pattern detection engine for Python codebases.
//!
//! Identifies code-quality anti-patterns that create maintenance friction
//! and impede automated reasoning:
//!
//! - **Magic Numbers:** Hard-coded numeric literals with 3+ digits used in logic,
//!   conditions, or comparisons rather than named constants.
//! - **Deeply Nested Callbacks:** Callback functions, closures, or control blocks
//!   nested 4+ levels deep (>= 16 spaces of indentation).

pub mod callbacks;
pub mod magic;
pub mod python;
pub mod reporter;
pub mod scanner;
pub mod types;

#[cfg(test)]
mod tests;

pub use python::detect_anti_patterns;
pub use reporter::{print_json_report, print_terminal_report};
pub use scanner::{scan_anti_patterns, scan_files};
pub use types::{AntiPatternKind, AntiPatternMatch, AntiPatternStats, AntiPatternsResult};

use std::path::PathBuf;

/// Analyzes anti-patterns across target root paths.
#[must_use]
pub fn analyze_anti_patterns(
    roots: &[PathBuf],
    exclude: &[String],
    verbose: bool,
) -> AntiPatternsResult {
    scan_anti_patterns(roots, exclude, verbose)
}

/// Analyzes anti-patterns across an explicit list of files.
#[must_use]
pub fn analyze_anti_patterns_files(files: &[PathBuf]) -> AntiPatternsResult {
    scan_files(files)
}
