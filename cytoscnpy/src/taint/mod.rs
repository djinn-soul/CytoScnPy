//! Taint Analysis Module
//!
//! Provides data flow-based taint analysis for detecting security vulnerabilities.
//! Tracks untrusted user input from sources to dangerous sinks.
//!
//! # Analysis Levels
//! - **Intraprocedural**: Within single functions
//! - **Interprocedural**: Across functions in same file
//! - **Cross-file**: Across modules

/// Taint analyzer core implementation.
pub mod analyzer;
/// Call graph construction for interprocedural analysis.
pub mod call_graph;
/// Cross-file taint analysis.
pub mod crossfile;
/// Interprocedural taint analysis logic.
pub mod interprocedural;
/// Intraprocedural (single function) taint analysis.
pub mod intraprocedural;
/// Taint propagation logic.
pub mod propagation;
/// Taint sink detection and classification.
pub mod sinks;
/// Taint source detection and management.
pub mod sources;
/// Taint summaries for functions.
pub mod summaries;
/// Common types used throughout taint analysis.
pub mod types;

pub use analyzer::TaintAnalyzer;
pub use types::{Severity, TaintFinding, TaintInfo, TaintSource, VulnType};
