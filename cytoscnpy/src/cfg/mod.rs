//! CFG (Control Flow Graph) module for behavioral validation and flow-sensitive analysis.
//!
//! This module provides CFG-based analysis for:
//! - Behavioral clone validation (secondary filter)
//! - Loop structure and branching shape fingerprinting
//! - Reachability analysis for dead code detection
//! - Data flow analysis (Reaching Definitions)
//!
//! # Feature Gate
//!
//! This module is only available with the `cfg` feature:
//! ```bash
//! cargo build --features cfg
//! ```
//!
//! # Design Principles
//!
//! - **One CFG per function**: Never cross function boundaries
//! - **Collapse straight-line blocks**: Simplify for faster analysis
//! - **Fingerprint only shape**: Loop structure, branching, call edges
//! - **NO compiler theory**: No SSA, no dominance trees (except simple worklist dataflow)
//!
//! # Note
//!
//! CFG is a **validator**, not a **detector**. Use it only as a secondary
//! filter for high-confidence clone groups when detection precision plateaus.

/// Data flow analysis and reaching definitions.
pub mod flow;

mod builder;
mod collector;
mod graph;
mod types;

pub use types::{BasicBlock, Cfg, CfgFingerprint, StmtKind, StmtRef};

#[cfg(test)]
mod tests;
