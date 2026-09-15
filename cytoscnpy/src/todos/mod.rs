//! TODO/FIXME/HACK/XXX placeholder, debug-print, and commented-code detection.
//!
//! Provides fast line-by-line scanning of Python source files for annotation
//! markers and code-quality anti-patterns without requiring AST parsing.
//!
//! ## Features
//! - Detects `TODO`, `FIXME`, `HACK`, and `XXX` markers (case-insensitive).
//! - Detects debug `print(` / `console.log(` / `println!(` in non-output code.
//! - Detects commented-out code blocks (`# if …`, `// for …`, …).
//! - Context-sensitive suppression: skips debug-print detection in test files
//!   and output-oriented modules (`cli`, `main`, `output`, `views`, …).
//! - Exposed as `cytoscnpy todos` CLI subcommand with `--json`,
//!   `--fail-on-any`, `-o`, and `--exclude`.

pub mod reporter;
pub mod scanner;
pub mod types;

#[cfg(test)]
mod tests;

pub use reporter::{print_json_report, print_terminal_report};
pub use scanner::{scan_file_content, scan_files, scan_todos};
pub use types::{TodoKind, TodoMatch, TodoStats, TodosResult};

use crate::commands::utils::find_python_files;
use std::path::PathBuf;

/// Analyzes TODO/annotation patterns across `roots`, respecting `exclude`
/// patterns and `.gitignore` rules.
#[must_use]
pub fn analyze_todos(roots: &[PathBuf], exclude: &[String], verbose: bool) -> TodosResult {
    let files = find_python_files(roots, exclude, verbose);
    scan_files(&files)
}

/// Analyzes an explicit list of Python file paths.
#[must_use]
pub fn analyze_todos_files(files: &[PathBuf]) -> TodosResult {
    scan_files(files)
}
