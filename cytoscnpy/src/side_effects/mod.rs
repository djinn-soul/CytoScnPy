//! Module-level import-time side-effects analysis for Python and `JavaScript`/`TypeScript`.
//!
//! Detects executable logic running at import time rather than inside functions
//! or classes, which leads to hidden dependencies, slow test suites, unpredictable
//! import ordering bugs, and startup friction.

pub mod polyglot;
pub mod python;
pub mod reporter;
pub mod scanner;
pub mod types;

#[cfg(test)]
mod tests;

pub use reporter::{print_json_report, print_terminal_report};
pub use scanner::{collect_files, is_test_file, scan_files};
pub use types::{SideEffectKind, SideEffectMatch, SideEffectStats, SideEffectsResult};

use std::path::PathBuf;

/// Analyzes module-level side effects across `roots` respecting `exclude` patterns.
#[must_use]
pub fn analyze_side_effects(
    roots: &[PathBuf],
    exclude: &[String],
    _verbose: bool,
) -> SideEffectsResult {
    let files = collect_files(roots, exclude);
    scan_files(&files)
}

/// Analyzes an explicit set of files for module-level side effects.
#[must_use]
pub fn analyze_side_effects_files(files: &[PathBuf]) -> SideEffectsResult {
    scan_files(files)
}
