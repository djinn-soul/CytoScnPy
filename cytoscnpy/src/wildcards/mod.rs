//! Wildcard-import (`from module import *`) detection.
//!
//! Provides Python AST-based scanning for `from … import *` statements,
//! which pollute the namespace, hide dependencies, and make refactoring risky.
//!
//! ## Features
//! - Detects every `from <module> import *` in Python source files.
//! - Recursive detection inside functions, class bodies, `if` branches,
//!   `for`/`while` loops, `with` blocks, `try`/`except`, and `match` cases.
//! - Captures the imported module name and source snippet.
//! - Parallel file scanning via Rayon.
//! - Exposed as `cytoscnpy wildcards` CLI subcommand with `--json`,
//!   `--fail-on-any`, `--max-wildcards`, `-o`, and `--exclude`.

pub mod reporter;
pub mod scanner;
pub mod types;

#[cfg(test)]
mod tests;

pub use reporter::{print_json_report, print_terminal_report};
pub use scanner::{detect_wildcard_imports, scan_files};
pub use types::{WildcardMatch, WildcardStats, WildcardsResult};

use crate::commands::utils::find_python_files;
use std::path::PathBuf;

/// Analyzes wildcard imports across `roots`, respecting `exclude` patterns
/// and `.gitignore` rules.
#[must_use]
pub fn analyze_wildcards(roots: &[PathBuf], exclude: &[String], verbose: bool) -> WildcardsResult {
    let files = find_python_files(roots, exclude, verbose);
    scan_files(&files)
}

/// Analyzes an explicit list of Python file paths.
#[must_use]
pub fn analyze_wildcards_files(files: &[PathBuf]) -> WildcardsResult {
    scan_files(files)
}
