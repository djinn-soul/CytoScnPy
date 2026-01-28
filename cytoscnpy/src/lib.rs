//! Core library for the CytoScnPy static analysis tool.
//!
//! This library provides the core functionality for analyzing Python code,
//! including AST parsing, visitor traversal, and rule execution.

// Allow common complexity warnings - these are intentional design choices
#![allow(
    clippy::type_complexity,
    clippy::too_many_arguments,
    clippy::ptr_arg,
    clippy::similar_names,
    clippy::format_push_string,
    clippy::map_unwrap_or,
    clippy::items_after_statements
)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::sync::atomic::AtomicBool;

/// Global cancellation flag for long-running operations.
/// This is set to true when a signal (like Ctrl+C) is detected.
pub static CANCELLED: AtomicBool = AtomicBool::new(false);

/// Module containing the core analyzer logic.
/// This includes the `CytoScnPy` struct and its methods for running the analysis.
pub mod analyzer;
pub mod report;

/// Module containing the AST visitor implementation.
/// This is responsible for traversing the Python AST and collecting data.
pub mod visitor;

/// Module defining the analysis result data structures.
/// This includes structs like `AnalysisResult`, `Finding`, `UnusedFunction`, etc.
pub mod framework;

/// Module for loading configuration.
pub mod config;

/// Module containing test utilities.
/// This helps in writing tests for the analyzer and rules.
pub mod test_utils;

/// Module containing the implementation of various analysis rules.
/// This includes rules for security, quality, and secrets.
pub mod rules;

/// Module containing utility functions.
/// This includes helper functions used across the application.
pub mod utils;

/// Module defining the entry point logic.
/// This handles the integration with Python's `setuptools/entry_points` ecosystem if needed.
pub mod entry_point;

/// Module containing shared constants and regex patterns.
pub mod constants;
/// Module containing the linter logic and visitor.
pub mod linter;

/// Module for rich CLI output formatting with colored text and spinners.
pub mod output;

/// Module defining the command-line interface arguments and structs.
pub mod cli;

/// Module for handling CLI commands and their execution logic.
pub mod commands;
/// Module for calculating cyclomatic complexity.
pub mod complexity;
/// Module for calculating Halstead metrics.
pub mod halstead;
/// Module for parsing and extracting code from Jupyter notebooks (.ipynb files).
pub mod ipynb;
/// Module defining traits and types for code metrics.
pub mod metrics;
/// Module for calculating raw code metrics (LOC, SLOC, etc.).
pub mod raw_metrics;
/// Module for taint analysis (data flow from sources to sinks).
pub mod taint;

/// Module for code clone detection and deduplication.
/// Provides Type-1/2/3 clone detection with safe `--fix` support.
pub mod clones;

/// Shared fix module for auto-remediation.
/// Provides reusable byte-range safe code rewriting for `--fix` support.
pub mod fix;

/// CST (Concrete Syntax Tree) module for precise source mapping.
/// Provides Tree-sitter based parsing for `--fix` with comment preservation.
/// Only available with the `cst` feature.
#[cfg(feature = "cst")]
pub mod cst;

/// CFG (Control Flow Graph) module for behavioral validation.
/// Provides secondary filtering for clone detection.
/// Only available with the `cfg` feature.
#[cfg(feature = "cfg")]
pub mod cfg;

/// Python bindings module (PyO3 integration).
/// Contains the implementation of Python-callable functions.
#[cfg(feature = "python-bindings")]
mod python_bindings;

// Re-export the Python module at the crate root (required by PyO3)
#[cfg(feature = "python-bindings")]
use pyo3::{pymodule, types::PyModule, Bound, PyResult};

/// Python module definition for `cytoscnpy`.
///
/// This is the entry point for Python imports. The actual implementation
/// is in the `python_bindings` module for better organization.
#[cfg(feature = "python-bindings")]
#[pymodule]
fn cytoscnpy(m: &Bound<'_, PyModule>) -> PyResult<()> {
    python_bindings::register_functions(m)
}
// Force rebuild
