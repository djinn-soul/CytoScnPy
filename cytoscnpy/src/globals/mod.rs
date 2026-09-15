//! Mutable global state analysis.
//!
//! Detects mutable global state patterns across Python and polyglot files:
//! - Module-level mutable collections (`LIST = []`, `DICT = {}`, `SET = set()`, etc.).
//! - Class-level mutable variables shared across all instances.
//! - Functions mutating module globals via `global`.
//! - Polyglot mutable globals: Rust `static mut` and JavaScript/TypeScript top-level mutable vars.
//! - Automatic test context suppression.
//! - Subcommand `cytoscnpy globals` with JSON output and CI threshold gating.

pub mod polyglot;
pub mod python;
pub mod reporter;
pub mod scanner;
pub mod types;

#[cfg(test)]
mod tests;

pub use reporter::{print_json_report, print_terminal_report};
pub use scanner::{collect_files, is_test_file, scan_files};
pub use types::{GlobalKind, GlobalMatch, GlobalStats, GlobalsResult};

use std::path::{Path, PathBuf};

/// Analyzes mutable global state across the given `roots` and exclusions.
#[must_use]
pub fn analyze_globals(roots: &[PathBuf], excludes: &[String], _verbose: bool) -> GlobalsResult {
    let files = collect_files(roots, excludes);
    let target = roots.first().cloned().unwrap_or_else(|| PathBuf::from("."));
    scan_files(&files, &target)
}

/// Analyzes mutable global state for an explicit list of files.
#[must_use]
pub fn analyze_globals_files(files: &[PathBuf], target: &Path) -> GlobalsResult {
    scan_files(files, target)
}
