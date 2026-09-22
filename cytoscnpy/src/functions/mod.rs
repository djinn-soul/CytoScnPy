//! Python function metric extraction and analysis.
//!
//! Extracts function names, locations, physical lengths, McCabe cyclomatic
//! complexity, and maximum control-flow nesting depth from Python source code
//! and aggregates codebase-wide function statistics.

pub mod extractor;
pub mod nesting;
pub mod reporter;
pub mod scanner;
pub mod types;

#[cfg(test)]
mod tests;

pub use extractor::{extract_functions, extract_functions_from_ast};
pub use nesting::calculate_max_nesting;
pub use reporter::{print_json_report, print_terminal_report};
pub use scanner::{analyze_functions, scan_files};
pub use types::{FunctionInfo, FunctionStats, FunctionsResult};
