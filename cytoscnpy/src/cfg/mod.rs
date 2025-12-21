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

use ruff_python_ast::{self as ast, Stmt};
use ruff_python_parser::parse_module;
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

impl BasicBlock {
    fn new(id: usize, loop_depth: usize) -> Self {
        Self {
            id,
            statements: Vec::new(),
            successors: Vec::new(),
            predecessors: Vec::new(),
            loop_depth,
        }
    }
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

/// Builder for constructing CFG from AST
struct CfgBuilder {
    blocks: Vec<BasicBlock>,
    current_block: usize,
    loop_depth: usize,
    /// Stack of (loop_header_id, loop_exit_id) for break/continue
    loop_stack: Vec<(usize, usize)>,
}

impl CfgBuilder {
    fn new() -> Self {
        let entry_block = BasicBlock::new(0, 0);
        Self {
            blocks: vec![entry_block],
            current_block: 0,
            loop_depth: 0,
            loop_stack: Vec::new(),
        }
    }

    fn new_block(&mut self) -> usize {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock::new(id, self.loop_depth));
        id
    }

    fn add_edge(&mut self, from: usize, to: usize) {
        if !self.blocks[from].successors.contains(&to) {
            self.blocks[from].successors.push(to);
        }
        if !self.blocks[to].predecessors.contains(&from) {
            self.blocks[to].predecessors.push(from);
        }
    }

    fn add_stmt(&mut self, kind: StmtKind, line: usize) {
        self.blocks[self.current_block]
            .statements
            .push(StmtRef { line, kind });
    }

    fn build_from_body(&mut self, body: &[Stmt]) {
        for stmt in body {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        use ruff_text_size::Ranged;

        let line = stmt.range().start().to_u32() as usize;

        match stmt {
            // Control flow altering statements
            Stmt::If(if_stmt) => {
                self.add_stmt(StmtKind::If, line);
                self.visit_if(if_stmt, line);
            }
            Stmt::For(for_stmt) => {
                self.add_stmt(StmtKind::For, line);
                self.visit_for(for_stmt);
            }
            Stmt::While(while_stmt) => {
                self.add_stmt(StmtKind::While, line);
                self.visit_while(while_stmt);
            }
            Stmt::Try(try_stmt) => {
                self.add_stmt(StmtKind::Try, line);
                self.visit_try(try_stmt);
            }
            Stmt::With(with_stmt) => {
                self.add_stmt(StmtKind::With, line);
                // With has a body but doesn't branch
                self.build_from_body(&with_stmt.body);
            }
            Stmt::Match(match_stmt) => {
                self.add_stmt(StmtKind::Match, line);
                self.visit_match(match_stmt);
            }

            // Terminators
            Stmt::Return(_) => {
                self.add_stmt(StmtKind::Return, line);
                // Return terminates this block - no successor needed here
                // The exit will be collected later
            }
            Stmt::Raise(_) => {
                self.add_stmt(StmtKind::Raise, line);
                // Raise terminates - similar to return
            }
            Stmt::Break(_) => {
                self.add_stmt(StmtKind::Break, line);
                // Jump to loop exit
                if let Some(&(_, exit_id)) = self.loop_stack.last() {
                    self.add_edge(self.current_block, exit_id);
                }
            }
            Stmt::Continue(_) => {
                self.add_stmt(StmtKind::Continue, line);
                // Jump back to loop header
                if let Some(&(header_id, _)) = self.loop_stack.last() {
                    self.add_edge(self.current_block, header_id);
                }
            }

            // Nested function/class definitions - don't recurse into them
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {
                self.add_stmt(StmtKind::Simple, line);
            }

            // Simple statements
            Stmt::Expr(expr_stmt) => {
                // Check if it's a call expression
                if matches!(expr_stmt.value.as_ref(), ast::Expr::Call(_)) {
                    self.add_stmt(StmtKind::Call, line);
                } else {
                    self.add_stmt(StmtKind::Simple, line);
                }
            }

            // All other statements are "simple"
            _ => {
                self.add_stmt(StmtKind::Simple, line);
            }
        }
    }

    fn visit_if(&mut self, if_stmt: &ast::StmtIf, _line: usize) {
        let before_block = self.current_block;

        // Create blocks for then branch
        let then_block = self.new_block();
        self.add_edge(before_block, then_block);

        // Build then body
        self.current_block = then_block;
        self.build_from_body(&if_stmt.body);
        let then_exit = self.current_block;

        // Handle elif/else clauses
        let mut branch_exits = vec![then_exit];
        let mut prev_block = before_block;

        for clause in &if_stmt.elif_else_clauses {
            let clause_block = self.new_block();
            self.add_edge(prev_block, clause_block);

            self.current_block = clause_block;
            self.build_from_body(&clause.body);
            branch_exits.push(self.current_block);

            // If this is an elif (has a test), the next clause branches from here
            if clause.test.is_some() {
                prev_block = clause_block;
            }
        }

        // If no else clause, add edge from condition block to merge
        let has_else = if_stmt
            .elif_else_clauses
            .last()
            .is_some_and(|c| c.test.is_none());
        if !has_else {
            branch_exits.push(prev_block);
        }

        // Create merge block
        let merge_block = self.new_block();
        for exit in branch_exits {
            self.add_edge(exit, merge_block);
        }
        self.current_block = merge_block;
    }

    fn visit_for(&mut self, for_stmt: &ast::StmtFor) {
        let before_block = self.current_block;

        // Loop header block
        let header_block = self.new_block();
        self.add_edge(before_block, header_block);

        // Exit block (after loop)
        let exit_block = self.new_block();

        // Push loop context for break/continue
        self.loop_stack.push((header_block, exit_block));
        self.loop_depth += 1;
        self.blocks[header_block].loop_depth = self.loop_depth;

        // Loop body
        let body_block = self.new_block();
        self.blocks[body_block].loop_depth = self.loop_depth;
        self.add_edge(header_block, body_block);

        self.current_block = body_block;
        self.build_from_body(&for_stmt.body);

        // Back edge to header
        self.add_edge(self.current_block, header_block);

        // Edge from header to exit (loop may not execute)
        self.add_edge(header_block, exit_block);

        // Handle else clause (runs if no break)
        if !for_stmt.orelse.is_empty() {
            let else_block = self.new_block();
            self.add_edge(header_block, else_block);
            self.current_block = else_block;
            self.build_from_body(&for_stmt.orelse);
            self.add_edge(self.current_block, exit_block);
        }

        self.loop_depth -= 1;
        self.loop_stack.pop();
        self.current_block = exit_block;
    }

    fn visit_while(&mut self, while_stmt: &ast::StmtWhile) {
        let before_block = self.current_block;

        // Loop header block (condition check)
        let header_block = self.new_block();
        self.add_edge(before_block, header_block);

        // Exit block
        let exit_block = self.new_block();

        // Push loop context
        self.loop_stack.push((header_block, exit_block));
        self.loop_depth += 1;
        self.blocks[header_block].loop_depth = self.loop_depth;

        // Loop body
        let body_block = self.new_block();
        self.blocks[body_block].loop_depth = self.loop_depth;
        self.add_edge(header_block, body_block);

        self.current_block = body_block;
        self.build_from_body(&while_stmt.body);

        // Back edge
        self.add_edge(self.current_block, header_block);

        // Exit edge (when condition is false)
        self.add_edge(header_block, exit_block);

        // Else clause
        if !while_stmt.orelse.is_empty() {
            let else_block = self.new_block();
            self.add_edge(header_block, else_block);
            self.current_block = else_block;
            self.build_from_body(&while_stmt.orelse);
            self.add_edge(self.current_block, exit_block);
        }

        self.loop_depth -= 1;
        self.loop_stack.pop();
        self.current_block = exit_block;
    }

    fn visit_try(&mut self, try_stmt: &ast::StmtTry) {
        let before_block = self.current_block;

        // Try body
        let try_block = self.new_block();
        self.add_edge(before_block, try_block);
        self.current_block = try_block;
        self.build_from_body(&try_stmt.body);
        let try_exit = self.current_block;

        // Exception handlers
        let mut handler_exits = vec![try_exit];
        for handler in &try_stmt.handlers {
            let handler_block = self.new_block();
            // Exception can be raised from try body
            self.add_edge(before_block, handler_block);

            self.current_block = handler_block;
            match handler {
                ast::ExceptHandler::ExceptHandler(h) => {
                    self.build_from_body(&h.body);
                }
            }
            handler_exits.push(self.current_block);
        }

        // Else clause (runs if no exception)
        if !try_stmt.orelse.is_empty() {
            let else_block = self.new_block();
            self.add_edge(try_exit, else_block);
            self.current_block = else_block;
            self.build_from_body(&try_stmt.orelse);
            handler_exits.push(self.current_block);
        }

        // Merge block (or finally)
        let merge_block = self.new_block();
        for exit in handler_exits {
            self.add_edge(exit, merge_block);
        }

        // Finally clause
        if !try_stmt.finalbody.is_empty() {
            self.current_block = merge_block;
            self.build_from_body(&try_stmt.finalbody);
        }

        self.current_block = merge_block;
    }

    fn visit_match(&mut self, match_stmt: &ast::StmtMatch) {
        let before_block = self.current_block;
        let mut case_exits = Vec::new();

        for case in &match_stmt.cases {
            let case_block = self.new_block();
            self.add_edge(before_block, case_block);
            self.current_block = case_block;
            self.build_from_body(&case.body);
            case_exits.push(self.current_block);
        }

        // Merge block
        let merge_block = self.new_block();
        for exit in case_exits {
            self.add_edge(exit, merge_block);
        }
        self.current_block = merge_block;
    }

    fn build(self) -> CFG {
        let entry = 0;

        // Find exit blocks (blocks with no successors or with return/raise)
        let exits: Vec<usize> = self
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, block)| {
                block.successors.is_empty()
                    || block
                        .statements
                        .last()
                        .is_some_and(|s| matches!(s.kind, StmtKind::Return | StmtKind::Raise))
            })
            .map(|(id, _)| id)
            .collect();

        CFG {
            blocks: self.blocks,
            entry,
            exits: if exits.is_empty() { vec![0] } else { exits },
        }
    }
}

impl CFG {
    /// Build a CFG from a function definition
    #[must_use]
    pub fn from_function(func: &ast::StmtFunctionDef) -> Self {
        let mut builder = CfgBuilder::new();
        builder.build_from_body(&func.body);
        builder.build()
    }

    /// Build a CFG from source code for a specific function name
    ///
    /// Returns None if the function is not found or parsing fails.
    #[must_use]
    pub fn from_source(source: &str, function_name: &str) -> Option<Self> {
        let parsed = parse_module(source).ok()?;
        let module = parsed.into_syntax();

        for stmt in &module.body {
            if let Stmt::FunctionDef(func) = stmt {
                if func.name.as_str() == function_name {
                    return Some(Self::from_function(func));
                }
            }
        }
        None
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

    /// Calculate a similarity score between 0.0 and 1.0
    #[must_use]
    pub fn similarity_score(&self, other: &Self) -> f64 {
        let fp1 = self.fingerprint();
        let fp2 = other.fingerprint();

        // Compare various metrics with weighted scoring
        let mut score = 0.0;
        let mut weight_sum = 0.0;

        // Block count comparison (weight: 2)
        let block_diff =
            (fp1.block_count as f64 - fp2.block_count as f64).abs() / fp1.block_count.max(1) as f64;
        score += (1.0 - block_diff.min(1.0)) * 2.0;
        weight_sum += 2.0;

        // Loop depth comparison (weight: 3)
        if fp1.max_loop_depth == fp2.max_loop_depth {
            score += 3.0;
        } else {
            let depth_diff = (fp1.max_loop_depth as f64 - fp2.max_loop_depth as f64).abs();
            score += (1.0 - depth_diff / 3.0).max(0.0) * 3.0;
        }
        weight_sum += 3.0;

        // Branch count comparison (weight: 2)
        if fp1.branch_count == fp2.branch_count {
            score += 2.0;
        } else {
            let branch_max = fp1.branch_count.max(fp2.branch_count).max(1) as f64;
            let branch_diff = (fp1.branch_count as f64 - fp2.branch_count as f64).abs();
            score += (1.0 - branch_diff / branch_max) * 2.0;
        }
        weight_sum += 2.0;

        // Loop count comparison (weight: 2)
        if fp1.loop_count == fp2.loop_count {
            score += 2.0;
        } else {
            let loop_max = fp1.loop_count.max(fp2.loop_count).max(1) as f64;
            let loop_diff = (fp1.loop_count as f64 - fp2.loop_count as f64).abs();
            score += (1.0 - loop_diff / loop_max) * 2.0;
        }
        weight_sum += 2.0;

        // Statement histogram similarity (weight: 1)
        let all_kinds: std::collections::HashSet<_> = fp1
            .stmt_histogram
            .keys()
            .chain(fp2.stmt_histogram.keys())
            .collect();
        if !all_kinds.is_empty() {
            let mut hist_match = 0.0;
            for kind in &all_kinds {
                let count1 = *fp1.stmt_histogram.get(kind).unwrap_or(&0) as f64;
                let count2 = *fp2.stmt_histogram.get(kind).unwrap_or(&0) as f64;
                let max_count = count1.max(count2).max(1.0);
                hist_match += 1.0 - (count1 - count2).abs() / max_count;
            }
            score += (hist_match / all_kinds.len() as f64) * 1.0;
        }
        weight_sum += 1.0;

        score / weight_sum
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

    #[test]
    fn test_cfg_from_source_simple() {
        let source = r#"
def simple_func():
    x = 1
    y = 2
    return x + y
"#;
        let cfg = CFG::from_source(source, "simple_func").expect("Should parse");

        // Simple linear function: entry block with statements
        assert!(!cfg.blocks.is_empty());
        let fp = cfg.fingerprint();
        assert_eq!(fp.branch_count, 0);
        assert_eq!(fp.loop_count, 0);
    }

    #[test]
    fn test_cfg_from_source_with_if() {
        let source = r#"
def func_with_if(x):
    if x > 0:
        return 1
    else:
        return -1
"#;
        let cfg = CFG::from_source(source, "func_with_if").expect("Should parse");

        let fp = cfg.fingerprint();
        assert_eq!(fp.branch_count, 1);
        assert_eq!(fp.loop_count, 0);
        // Should have multiple blocks due to branching
        assert!(fp.block_count > 1);
    }

    #[test]
    fn test_cfg_from_source_with_loop() {
        let source = r#"
def func_with_loop():
    total = 0
    for i in range(10):
        total += i
    return total
"#;
        let cfg = CFG::from_source(source, "func_with_loop").expect("Should parse");

        let fp = cfg.fingerprint();
        assert_eq!(fp.loop_count, 1);
        assert_eq!(fp.max_loop_depth, 1);
    }

    #[test]
    fn test_cfg_from_source_nested_loops() {
        let source = r#"
def nested_loops():
    for i in range(10):
        for j in range(10):
            print(i, j)
"#;
        let cfg = CFG::from_source(source, "nested_loops").expect("Should parse");

        let fp = cfg.fingerprint();
        assert_eq!(fp.loop_count, 2);
        assert_eq!(fp.max_loop_depth, 2);
    }

    #[test]
    fn test_cfg_behavioral_similarity_identical() {
        let source = r#"
def func_a():
    if True:
        x = 1
    for i in range(10):
        print(i)

def func_b():
    if False:
        y = 2
    for j in range(20):
        print(j)
"#;
        let cfg_a = CFG::from_source(source, "func_a").expect("Should parse");
        let cfg_b = CFG::from_source(source, "func_b").expect("Should parse");

        // Same structure = behaviorally similar
        assert!(cfg_a.is_behaviorally_similar(&cfg_b));
        assert!(cfg_a.similarity_score(&cfg_b) > 0.9);
    }

    #[test]
    fn test_cfg_behavioral_similarity_different() {
        let source = r#"
def simple_func():
    return 1

def complex_func():
    for i in range(10):
        if i > 5:
            return i
    return 0
"#;
        let cfg_simple = CFG::from_source(source, "simple_func").expect("Should parse");
        let cfg_complex = CFG::from_source(source, "complex_func").expect("Should parse");

        // Different structure = not behaviorally similar
        assert!(!cfg_simple.is_behaviorally_similar(&cfg_complex));
        assert!(cfg_simple.similarity_score(&cfg_complex) < 0.8);
    }

    #[test]
    fn test_cfg_with_try_except() {
        let source = r#"
def func_with_try():
    try:
        risky_call()
    except ValueError:
        handle_error()
    finally:
        cleanup()
"#;
        let cfg = CFG::from_source(source, "func_with_try").expect("Should parse");

        let fp = cfg.fingerprint();
        assert!(fp.stmt_histogram.get(&StmtKind::Try).is_some());
        // Try creates multiple blocks for handlers
        assert!(fp.block_count > 1);
    }

    #[test]
    fn test_cfg_function_not_found() {
        let source = "def other_func(): pass";
        let result = CFG::from_source(source, "nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_cfg_with_while_break_continue() {
        let source = r#"
def func_with_control():
    i = 0
    while i < 10:
        i += 1
        if i == 5:
            continue
        if i == 8:
            break
    return i
"#;
        let cfg = CFG::from_source(source, "func_with_control").expect("Should parse");

        let fp = cfg.fingerprint();
        assert_eq!(fp.loop_count, 1);
        assert_eq!(fp.branch_count, 2); // Two if statements
        assert!(fp.stmt_histogram.get(&StmtKind::Break).is_some());
        assert!(fp.stmt_histogram.get(&StmtKind::Continue).is_some());
    }
}
