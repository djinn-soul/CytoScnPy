//! Python singleton pattern detection.
//!
//! Detects singleton patterns such as `__new__` instance caching, `_instance`
//! attribute with `get_instance()` accessors, `@singleton` decorators, and
//! `metaclass=Singleton`. Singletons hide global state, hinder modularity,
//! and introduce friction during unit testing and concurrent execution.

pub mod python;
pub mod reporter;
pub mod scanner;
pub mod types;

#[cfg(test)]
mod tests;

pub use python::detect_python_singletons;
pub use reporter::{print_json_report, print_terminal_report};
pub use scanner::scan_files;
pub use types::{SingletonKind, SingletonMatch, SingletonStats, SingletonsResult};

use crate::commands::utils::find_python_files;
use std::path::PathBuf;

/// Analyzes Python singleton patterns across `roots` respecting `exclude` patterns.
#[must_use]
pub fn analyze_singletons(
    roots: &[PathBuf],
    exclude: &[String],
    verbose: bool,
) -> SingletonsResult {
    let files = find_python_files(roots, exclude, verbose);
    scan_files(&files)
}

/// Analyzes an explicit list of Python file paths.
#[must_use]
pub fn analyze_singletons_files(files: &[PathBuf]) -> SingletonsResult {
    scan_files(files)
}
