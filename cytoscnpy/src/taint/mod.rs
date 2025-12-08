//! Taint Analysis Module
//!
//! Provides data flow-based taint analysis for detecting security vulnerabilities.
//! Tracks untrusted user input from sources to dangerous sinks.
//!
//! # Analysis Levels
//! - **Intraprocedural**: Within single functions
//! - **Interprocedural**: Across functions in same file
//! - **Cross-file**: Across modules

pub mod analyzer;
pub mod call_graph;
pub mod crossfile;
pub mod interprocedural;
pub mod intraprocedural;
pub mod propagation;
pub mod sinks;
pub mod sources;
pub mod summaries;
pub mod types;

pub use analyzer::TaintAnalyzer;
pub use types::{Severity, TaintFinding, TaintInfo, TaintSource, VulnType};
