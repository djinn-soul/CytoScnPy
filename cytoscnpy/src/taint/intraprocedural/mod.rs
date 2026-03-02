//! Intraprocedural taint analysis.
//!
//! Analyzes data flow within a single function.

pub(crate) mod entry;
mod handlers;
mod sinks;

pub use entry::{analyze_async_function, analyze_function, analyze_stmt_public};
