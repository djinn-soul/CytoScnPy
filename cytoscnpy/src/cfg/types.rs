use rustc_hash::FxHashSet;
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
    /// Variables defined in this block (Name, Line)
    pub defs: FxHashSet<(String, usize)>,
    /// Variables used in this block (Name, Line)
    pub uses: FxHashSet<(String, usize)>,
}

impl BasicBlock {
    pub(super) fn new(id: usize, loop_depth: usize) -> Self {
        Self {
            id,
            statements: Vec::new(),
            successors: Vec::new(),
            predecessors: Vec::new(),
            loop_depth,
            defs: FxHashSet::default(),
            uses: FxHashSet::default(),
        }
    }
}

/// Control Flow Graph for a single function
#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
pub struct Cfg {
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
