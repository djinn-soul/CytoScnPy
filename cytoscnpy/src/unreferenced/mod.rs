//! Unreferenced large functions in isolated files analysis.
//!
//! Detects functions with substantial line counts in files that have no incoming
//! imports or references, and whose names are never referenced across the codebase.

pub mod analyzer;
pub mod extractor;
pub mod heuristics;
pub mod reporter;
pub mod types;

#[cfg(test)]
mod tests;

pub use analyzer::{analyze_unreferenced, analyze_unreferenced_files};
pub use heuristics::UnreferencedOptions;
pub use reporter::{print_json_report, print_terminal_report};
pub use types::{IsolatedFileSummary, UnreferencedFunction, UnreferencedResult, UnreferencedStats};
