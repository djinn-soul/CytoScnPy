//! CFG (Control Flow Graph) module for behavioral validation.
//!
//! This module provides CFG-based analysis for:
//! - Behavioral clone validation (secondary filter)
//! - Loop structure and branching shape fingerprinting
//! - Reachability analysis for dead code detection
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
//! - **NO compiler theory**: No SSA, no dominance trees
//!
//! # Note
//!
//! CFG is a **validator**, not a **detector**. Use it only as a secondary
//! filter for high-confidence clone groups when detection precision plateaus.

use ruff_python_ast as ast;
use std::collections::HashMap;

/// Reference to a statement in the original AST
#[derive(Debug, Clone)]
pub struct StmtRef {
    /// Line number (1-indexed)
    pub line: usize,
    /// Statement kind for fingerprinting
    pub kind: StmtKind,
}

/// Simplified statement kinds for CFG fingerprinting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StmtKind {
    /// Assignment or expression
    Simple,
    /// If statement
    If,
    /// For loop
    For,
    /// While loop
    While,
    /// Return statement
    Return,
    /// Raise statement
    Raise,
    /// Break statement
    Break,
    /// Continue statement
    Continue,
    /// Try block
    Try,
    /// With statement
    With,
    /// Match statement (Python 3.10+)
    Match,
    /// Function call
    Call,
}

/// A basic block in the CFG
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// Unique block ID
    pub id: usize,
    /// Statements in this block
    pub statements: Vec<StmtRef>,
    /// Successor block IDs
    pub successors: Vec<usize>,
    /// Predecessor block IDs
    pub predecessors: Vec<usize>,
    /// Loop nesting depth
    pub loop_depth: usize,
}

/// Control Flow Graph for a single function
#[derive(Debug)]
pub struct CFG {
    /// Basic blocks indexed by ID
    pub blocks: Vec<BasicBlock>,
    /// Entry block ID
    pub entry: usize,
    /// Exit block IDs
    pub exits: Vec<usize>,
}

/// CFG fingerprint for behavioral comparison
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfgFingerprint {
    /// Number of basic blocks
    pub block_count: usize,
    /// Maximum loop depth
    pub max_loop_depth: usize,
    /// Number of branches (if/match)
    pub branch_count: usize,
    /// Number of loops (for/while)
    pub loop_count: usize,
    /// Statement kind histogram
    pub stmt_histogram: HashMap<StmtKind, usize>,
}

impl CFG {
    /// Build a CFG from a function definition
    ///
    /// TODO: Implement full CFG construction
    /// Current implementation is a stub for the feature gate
    #[must_use]
    pub fn from_function(_func: &ast::StmtFunctionDef) -> Self {
        // Stub implementation - will be fully implemented when needed
        Self {
            blocks: vec![BasicBlock {
                id: 0,
                statements: Vec::new(),
                successors: Vec::new(),
                predecessors: Vec::new(),
                loop_depth: 0,
            }],
            entry: 0,
            exits: vec![0],
        }
    }

    /// Generate a behavioral fingerprint from this CFG
    #[must_use]
    pub fn fingerprint(&self) -> CfgFingerprint {
        let mut stmt_histogram = HashMap::new();
        let mut max_loop_depth = 0;
        let mut branch_count = 0;
        let mut loop_count = 0;

        for block in &self.blocks {
            max_loop_depth = max_loop_depth.max(block.loop_depth);

            for stmt in &block.statements {
                *stmt_histogram.entry(stmt.kind).or_insert(0) += 1;

                match stmt.kind {
                    StmtKind::If | StmtKind::Match => branch_count += 1,
                    StmtKind::For | StmtKind::While => loop_count += 1,
                    _ => {}
                }
            }
        }

        CfgFingerprint {
            block_count: self.blocks.len(),
            max_loop_depth,
            branch_count,
            loop_count,
            stmt_histogram,
        }
    }

    /// Check if two CFGs have similar behavioral shape
    #[must_use]
    pub fn is_behaviorally_similar(&self, other: &Self) -> bool {
        let fp1 = self.fingerprint();
        let fp2 = other.fingerprint();

        // Simple similarity: same structure counts
        fp1.block_count == fp2.block_count
            && fp1.max_loop_depth == fp2.max_loop_depth
            && fp1.branch_count == fp2.branch_count
            && fp1.loop_count == fp2.loop_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfg_fingerprint() {
        let cfg = CFG {
            blocks: vec![BasicBlock {
                id: 0,
                statements: vec![
                    StmtRef {
                        line: 1,
                        kind: StmtKind::If,
                    },
                    StmtRef {
                        line: 2,
                        kind: StmtKind::For,
                    },
                ],
                successors: vec![],
                predecessors: vec![],
                loop_depth: 1,
            }],
            entry: 0,
            exits: vec![0],
        };

        let fp = cfg.fingerprint();
        assert_eq!(fp.block_count, 1);
        assert_eq!(fp.max_loop_depth, 1);
        assert_eq!(fp.branch_count, 1);
        assert_eq!(fp.loop_count, 1);
    }
}
