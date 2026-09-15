//! Naming style distribution, consistency analysis, and outlier detection.
//!
//! Provides static analysis of Python identifier naming conventions:
//! - Classifies function and method identifiers into `snake_case`, `camelCase`, `PascalCase`,
//!   `SCREAMING_SNAKE_CASE`, or `mixed`.
//! - Python-aware handling: trims leading private/mangled underscores (`_private`, `__mangled`),
//!   trims trailing keyword-collision underscores (`class_`), exempts structural dunder methods (`__init__`)
//!   and unittest fixtures (`setUp`), and ignores anonymous/empty definitions.
//! - Computes dominant style and consistency ratios.
//! - Extracts outliers deviating from the dominant codebase style.

pub mod classifier;
pub mod reporter;
pub mod scanner;
pub mod types;

#[cfg(test)]
mod tests;

pub use classifier::{
    classify_identifier, is_anonymous_or_empty, is_dunder_name, is_unittest_fixture,
    UNITTEST_FIXTURE_NAMES,
};
pub use reporter::{print_json_report, print_terminal_report};
pub use scanner::{analyze_naming_from_source, analyze_naming_functions, scan_and_analyze_naming};
pub use types::{
    normalize_consistency_threshold, NamingDistributionResult, NamingOutlier, NamingStats,
    NamingStyle,
};

use crate::commands::utils::find_python_files;
use std::path::PathBuf;

/// Analyzes naming style distribution across target roots.
#[must_use]
pub fn analyze_naming(
    roots: &[PathBuf],
    exclude: &[String],
    verbose: bool,
) -> NamingDistributionResult {
    let files = find_python_files(roots, exclude, verbose);
    analyze_naming_files(&files)
}

/// Analyzes naming style distribution over an explicit slice of Python file paths.
#[must_use]
pub fn analyze_naming_files(files: &[PathBuf]) -> NamingDistributionResult {
    scan_and_analyze_naming(files)
}
