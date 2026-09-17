//! Bare-except and empty exception-handler detection.
//!
//! Provides Python AST-based scanning for two related exception-handling
//! anti-patterns that silently swallow errors:
//!
//! ## Features
//! - Detects `except:` with no exception type (`BareExcept`).
//! - Detects handlers whose body consists only of `pass`, `...`, or a bare
//!   string literal (`EmptyHandler`).
//! - Recursive detection inside functions, class methods, nested `try` blocks,
//!   `if` branches, `for`/`while` loops, `with` blocks, and `match` cases.
//! - Exposed as `cytoscnpy exceptions` CLI subcommand with `--json`,
//!   `--fail-on-any`, `--max-bare-excepts`, `--max-empty-handlers`, `-o`,
//!   and `--exclude`.

pub mod reporter;
pub mod scanner;
pub mod types;

#[cfg(test)]
mod tests;

pub use reporter::{print_json_report, print_terminal_report};
pub use scanner::{detect_python_exceptions, scan_files};
pub use types::{ExceptionKind, ExceptionMatch, ExceptionStats, ExceptionsResult};

use crate::commands::utils::find_python_files;
use std::path::PathBuf;

/// Analyzes exception-handler anti-patterns across `roots`, respecting
/// `exclude` patterns and `.gitignore` rules.
#[must_use]
pub fn analyze_exceptions(
    roots: &[PathBuf],
    exclude: &[String],
    verbose: bool,
) -> ExceptionsResult {
    let files = find_python_files(roots, exclude, verbose);
    scan_files(&files)
}

/// Analyzes an explicit list of Python file paths.
///
/// This is a library-facing entry point. It has no internal CLI callers;
/// consumers embedding `cytoscnpy` as a library can use it to scan a
/// pre-collected file list without the CLI path-resolution layer.
#[must_use]
pub fn analyze_exceptions_files(files: &[PathBuf]) -> ExceptionsResult {
    scan_files(files)
}
