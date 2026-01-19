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

use crate::utils::LineIndex;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Stmt};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;
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
    fn new(id: usize, loop_depth: usize) -> Self {
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

/// Collector for variable definitions and usages in a block
struct NameCollector<'a> {
    defs: &'a mut FxHashSet<(String, usize)>,
    uses: &'a mut FxHashSet<(String, usize)>,
    current_line: usize,
}

impl<'a> Visitor<'a> for NameCollector<'a> {
    fn visit_expr(&mut self, expr: &'a ast::Expr) {
        match expr {
            ast::Expr::Name(name) => match name.ctx {
                ast::ExprContext::Load => {
                    self.uses.insert((name.id.to_string(), self.current_line));
                }
                ast::ExprContext::Store | ast::ExprContext::Del => {
                    self.defs.insert((name.id.to_string(), self.current_line));
                }
                ast::ExprContext::Invalid => {}
            },
            _ => visitor::walk_expr(self, expr),
        }
    }

    fn visit_stmt(&mut self, stmt: &'a ast::Stmt) {
        match stmt {
            ast::Stmt::FunctionDef(func) => {
                self.defs.insert((func.name.to_string(), self.current_line));
            }
            ast::Stmt::ClassDef(class) => {
                self.defs
                    .insert((class.name.to_string(), self.current_line));
            }
            ast::Stmt::Assign(assign) => {
                for target in &assign.targets {
                    self.visit_expr(target);
                }
                self.visit_expr(&assign.value);
            }
            ast::Stmt::AnnAssign(assign) => {
                self.visit_expr(&assign.target);
                if let Some(value) = &assign.value {
                    self.visit_expr(value);
                }
            }
            ast::Stmt::AugAssign(assign) => {
                self.visit_expr(&assign.target);
                self.visit_expr(&assign.value);
            }
            ast::Stmt::Expr(expr) => {
                self.visit_expr(&expr.value);
            }
            ast::Stmt::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.visit_expr(value);
                }
            }
            ast::Stmt::Raise(raise) => {
                if let Some(exc) = &raise.exc {
                    self.visit_expr(exc);
                }
                if let Some(cause) = &raise.cause {
                    self.visit_expr(cause);
                }
            }
            ast::Stmt::Assert(assert) => {
                self.visit_expr(&assert.test);
                if let Some(msg) = &assert.msg {
                    self.visit_expr(msg);
                }
            }
            ast::Stmt::Delete(delete) => {
                for target in &delete.targets {
                    self.visit_expr(target);
                }
            }
            _ => {}
        }
    }

    fn visit_pattern(&mut self, pattern: &'a ast::Pattern) {
        match pattern {
            ast::Pattern::MatchAs(p) => {
                if let Some(name) = &p.name {
                    self.defs.insert((name.to_string(), self.current_line));
                }
                if let Some(pattern) = &p.pattern {
                    self.visit_pattern(pattern);
                }
            }
            ast::Pattern::MatchMapping(p) => {
                if let Some(rest) = &p.rest {
                    self.defs.insert((rest.to_string(), self.current_line));
                }
                for key in &p.keys {
                    self.visit_expr(key);
                }
                for pattern in &p.patterns {
                    self.visit_pattern(pattern);
                }
            }
            ast::Pattern::MatchStar(p) => {
                if let Some(name) = &p.name {
                    self.defs.insert((name.to_string(), self.current_line));
                }
            }
            _ => visitor::walk_pattern(self, pattern),
        }
    }
}

/// Builder for constructing CFG from AST
struct CfgBuilder<'a> {
    blocks: Vec<BasicBlock>,
    current_block: usize,
    loop_depth: usize,
    /// Stack of (`loop_header_id`, `loop_exit_id`) for break/continue
    loop_stack: Vec<(usize, usize)>,
    line_index: &'a LineIndex,
}

impl<'a> CfgBuilder<'a> {
    fn new(line_index: &'a LineIndex) -> Self {
        let entry_block = BasicBlock::new(0, 0);
        Self {
            blocks: vec![entry_block],
            current_block: 0,
            loop_depth: 0,
            loop_stack: Vec::new(),
            line_index,
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

    fn build_from_function(&mut self, func: &ast::StmtFunctionDef) {
        // Collect parameters as definitions in the entry block
        for arg in &func.parameters.posonlyargs {
            let name = arg.parameter.name.to_string();
            let line = self.line_index.line_index(arg.parameter.range().start());
            self.blocks[0].defs.insert((name, line));
        }
        for arg in &func.parameters.args {
            let name = arg.parameter.name.to_string();
            let line = self.line_index.line_index(arg.parameter.range().start());
            self.blocks[0].defs.insert((name, line));
        }
        if let Some(arg) = &func.parameters.vararg {
            let name = arg.name.to_string();
            let line = self.line_index.line_index(arg.range().start());
            self.blocks[0].defs.insert((name, line));
        }
        for arg in &func.parameters.kwonlyargs {
            let name = arg.parameter.name.to_string();
            let line = self.line_index.line_index(arg.parameter.range().start());
            self.blocks[0].defs.insert((name, line));
        }
        if let Some(arg) = &func.parameters.kwarg {
            let name = arg.name.to_string();
            let line = self.line_index.line_index(arg.range().start());
            self.blocks[0].defs.insert((name, line));
        }

        for stmt in &func.body {
            self.visit_stmt(stmt);
        }
    }

    fn build_from_body(&mut self, body: &[Stmt]) {
        for stmt in body {
            self.visit_stmt(stmt);
        }
    }

    fn collect_expr_names(&mut self, expr: &ast::Expr, line: usize) {
        let block = &mut self.blocks[self.current_block];
        let mut collector = NameCollector {
            defs: &mut block.defs,
            uses: &mut block.uses,
            current_line: line,
        };
        collector.visit_expr(expr);
    }

    fn collect_pattern_names(&mut self, pattern: &ast::Pattern, line: usize) {
        let block = &mut self.blocks[self.current_block];
        let mut collector = NameCollector {
            defs: &mut block.defs,
            uses: &mut block.uses,
            current_line: line,
        };
        collector.visit_pattern(pattern);
    }

    fn collect_stmt_names(&mut self, stmt: &Stmt, line: usize) {
        let block = &mut self.blocks[self.current_block];
        let mut collector = NameCollector {
            defs: &mut block.defs,
            uses: &mut block.uses,
            current_line: line,
        };
        collector.visit_stmt(stmt);
    }

    #[allow(clippy::match_same_arms)]
    fn visit_stmt(&mut self, stmt: &Stmt) {
        use ruff_text_size::Ranged;
        let line = self.line_index.line_index(stmt.range().start());

        match stmt {
            Stmt::If(if_stmt) => {
                self.collect_expr_names(&if_stmt.test, line);
                self.add_stmt(StmtKind::If, line);
                self.visit_if(if_stmt);
            }
            Stmt::For(for_stmt) => {
                self.collect_expr_names(&for_stmt.target, line);
                self.collect_expr_names(&for_stmt.iter, line);
                self.add_stmt(StmtKind::For, line);
                self.visit_for(for_stmt);
            }
            Stmt::While(while_stmt) => {
                self.collect_expr_names(&while_stmt.test, line);
                self.add_stmt(StmtKind::While, line);
                self.visit_while(while_stmt);
            }
            Stmt::Try(try_stmt) => {
                self.add_stmt(StmtKind::Try, line);
                self.visit_try(try_stmt);
            }
            Stmt::With(with_stmt) => {
                for item in &with_stmt.items {
                    self.collect_expr_names(&item.context_expr, line);
                    if let Some(optional_vars) = &item.optional_vars {
                        self.collect_expr_names(optional_vars, line);
                    }
                }
                self.add_stmt(StmtKind::With, line);
                self.build_from_body(&with_stmt.body);
            }
            Stmt::Match(match_stmt) => {
                self.collect_expr_names(&match_stmt.subject, line);
                self.add_stmt(StmtKind::Match, line);
                self.visit_match(match_stmt, line);
            }
            _ => {
                self.collect_stmt_names(stmt, line);
                match stmt {
                    Stmt::Return(_) => self.add_stmt(StmtKind::Return, line),
                    Stmt::Raise(_) => self.add_stmt(StmtKind::Raise, line),
                    Stmt::Break(_) => {
                        self.add_stmt(StmtKind::Break, line);
                        if let Some(&(_, exit_id)) = self.loop_stack.last() {
                            self.add_edge(self.current_block, exit_id);
                        }
                    }
                    Stmt::Continue(_) => {
                        self.add_stmt(StmtKind::Continue, line);
                        if let Some(&(header_id, _)) = self.loop_stack.last() {
                            self.add_edge(self.current_block, header_id);
                        }
                    }
                    Stmt::Expr(expr_stmt) => {
                        if matches!(expr_stmt.value.as_ref(), ast::Expr::Call(_)) {
                            self.add_stmt(StmtKind::Call, line);
                        } else {
                            self.add_stmt(StmtKind::Simple, line);
                        }
                    }
                    _ => self.add_stmt(StmtKind::Simple, line),
                }
            }
        }
    }

    fn visit_if(&mut self, if_stmt: &ast::StmtIf) {
        let before_block = self.current_block;
        let then_block = self.new_block();
        self.add_edge(before_block, then_block);

        self.current_block = then_block;
        self.build_from_body(&if_stmt.body);
        let then_exit = self.current_block;

        let mut branch_exits = vec![then_exit];
        let mut prev_block = before_block;

        for clause in &if_stmt.elif_else_clauses {
            let clause_block = self.new_block();
            self.add_edge(prev_block, clause_block);

            if let Some(test) = &clause.test {
                // For elif, the test happens in the condition block
                self.current_block = clause_block;
                let line = self.line_index.line_index(clause.range().start());
                self.collect_expr_names(test, line);
            }

            self.current_block = clause_block;
            self.build_from_body(&clause.body);
            branch_exits.push(self.current_block);

            if clause.test.is_some() {
                prev_block = clause_block;
            }
        }

        let has_else = if_stmt
            .elif_else_clauses
            .last()
            .is_some_and(|c| c.test.is_none());
        if !has_else {
            branch_exits.push(prev_block);
        }

        let merge_block = self.new_block();
        for exit in branch_exits {
            self.add_edge(exit, merge_block);
        }
        self.current_block = merge_block;
    }

    fn visit_for(&mut self, for_stmt: &ast::StmtFor) {
        let before_block = self.current_block;
        let header_block = self.new_block();
        self.add_edge(before_block, header_block);
        let exit_block = self.new_block();

        self.loop_stack.push((header_block, exit_block));
        self.loop_depth += 1;
        self.blocks[header_block].loop_depth = self.loop_depth;

        let body_block = self.new_block();
        self.blocks[body_block].loop_depth = self.loop_depth;
        self.add_edge(header_block, body_block);

        self.current_block = body_block;
        self.build_from_body(&for_stmt.body);
        self.add_edge(self.current_block, header_block);
        self.add_edge(header_block, exit_block);

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
        let header_block = self.new_block();
        self.add_edge(before_block, header_block);
        let exit_block = self.new_block();

        self.loop_stack.push((header_block, exit_block));
        self.loop_depth += 1;
        self.blocks[header_block].loop_depth = self.loop_depth;

        let body_block = self.new_block();
        self.blocks[body_block].loop_depth = self.loop_depth;
        self.add_edge(header_block, body_block);

        self.current_block = body_block;
        self.build_from_body(&while_stmt.body);
        self.add_edge(self.current_block, header_block);
        self.add_edge(header_block, exit_block);

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
        let try_block = self.new_block();
        self.add_edge(before_block, try_block);
        self.current_block = try_block;
        self.build_from_body(&try_stmt.body);
        let try_exit = self.current_block;

        let mut handler_exits = vec![try_exit];
        for handler in &try_stmt.handlers {
            let handler_block = self.new_block();
            self.add_edge(before_block, handler_block);
            self.current_block = handler_block;
            match handler {
                ast::ExceptHandler::ExceptHandler(h) => {
                    if let Some(name) = &h.name {
                        self.blocks[handler_block]
                            .defs
                            .insert((name.to_string(), 0));
                    }
                    if let Some(type_expr) = &h.type_ {
                        self.collect_expr_names(type_expr, 0);
                    }
                    self.build_from_body(&h.body);
                }
            }
            handler_exits.push(self.current_block);
        }

        if !try_stmt.orelse.is_empty() {
            let else_block = self.new_block();
            self.add_edge(try_exit, else_block);
            self.current_block = else_block;
            self.build_from_body(&try_stmt.orelse);
            handler_exits.push(self.current_block);
        }

        let merge_block = self.new_block();
        for exit in handler_exits {
            self.add_edge(exit, merge_block);
        }

        if !try_stmt.finalbody.is_empty() {
            self.current_block = merge_block;
            self.build_from_body(&try_stmt.finalbody);
        }
        self.current_block = merge_block;
    }

    fn visit_match(&mut self, match_stmt: &ast::StmtMatch, _line: usize) {
        use ruff_text_size::Ranged;
        let before_block = self.current_block;
        let mut case_exits = Vec::new();

        for case in &match_stmt.cases {
            let case_line = self.line_index.line_index(case.range().start());

            // 1. Pattern matching block (definitions happen here)
            let pattern_block = self.new_block();
            self.add_edge(before_block, pattern_block);
            self.current_block = pattern_block;
            self.collect_pattern_names(&case.pattern, case_line);

            let mut branch_start = pattern_block;

            // 2. Optional guard block (uses happen here)
            if let Some(guard) = &case.guard {
                let guard_block = self.new_block();
                self.add_edge(pattern_block, guard_block);
                self.current_block = guard_block;
                self.collect_expr_names(guard, case_line);
                branch_start = guard_block;
            }

            // 3. Case body block
            let body_block = self.new_block();
            self.add_edge(branch_start, body_block);
            self.current_block = body_block;
            self.build_from_body(&case.body);
            case_exits.push(self.current_block);
        }

        let merge_block = self.new_block();
        for exit in case_exits {
            self.add_edge(exit, merge_block);
        }
        self.current_block = merge_block;
    }

    fn build(self) -> Cfg {
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

        Cfg {
            blocks: self.blocks,
            entry: 0,
            exits: if exits.is_empty() { vec![0] } else { exits },
        }
    }
}

impl Cfg {
    /// Constructs a CFG from a function AST node and its line index.
    pub fn from_function(func: &ast::StmtFunctionDef, line_index: &LineIndex) -> Self {
        let mut builder = CfgBuilder::new(line_index);
        builder.build_from_function(func);
        builder.build()
    }

    /// Constructs a CFG from a function's source code and its name.
    #[must_use]
    pub fn from_source(source: &str, function_name: &str) -> Option<Self> {
        let parsed = parse_module(source).ok()?;
        let module = parsed.into_syntax();
        let line_index = LineIndex::new(source);
        for stmt in &module.body {
            if let Stmt::FunctionDef(func) = stmt {
                if func.name.as_str() == function_name {
                    return Some(Self::from_function(func, &line_index));
                }
            }
        }
        None
    }

    /// Generates a fingerprint representing the control flow of this graph.
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

    /// Checks if two fingerprints are behaviorally similar.
    #[must_use]
    pub fn is_behaviorally_similar(&self, other: &Self) -> bool {
        let fp1 = self.fingerprint();
        let fp2 = other.fingerprint();
        fp1.block_count == fp2.block_count
            && fp1.max_loop_depth == fp2.max_loop_depth
            && fp1.branch_count == fp2.branch_count
            && fp1.loop_count == fp2.loop_count
    }

    #[allow(clippy::cast_precision_loss)]
    /// Calculates the similarity score between two fingerprints (0.0 to 1.0).
    #[must_use]
    pub fn similarity_score(&self, other: &Self) -> f64 {
        let fp1 = self.fingerprint();
        let fp2 = other.fingerprint();
        let mut score = 0.0;
        let mut weight_sum = 0.0;

        let block_diff =
            (fp1.block_count as f64 - fp2.block_count as f64).abs() / fp1.block_count.max(1) as f64;
        score += (1.0 - block_diff.min(1.0)) * 2.0;
        weight_sum += 2.0;

        if fp1.max_loop_depth == fp2.max_loop_depth {
            score += 3.0;
        } else {
            score += (1.0
                - (fp1.max_loop_depth as f64 - fp2.max_loop_depth as f64).abs() / 3.0_f64)
                .max(0.0)
                * 3.0;
        }
        weight_sum += 3.0;

        if fp1.branch_count == fp2.branch_count {
            score += 2.0;
        } else {
            score += (1.0
                - (fp1.branch_count as f64 - fp2.branch_count as f64).abs()
                    / fp1.branch_count.max(fp2.branch_count).max(1) as f64)
                * 2.0;
        }
        weight_sum += 2.0;

        if fp1.loop_count == fp2.loop_count {
            score += 2.0;
        } else {
            score += (1.0
                - (fp1.loop_count as f64 - fp2.loop_count as f64).abs()
                    / fp1.loop_count.max(fp2.loop_count).max(1) as f64)
                * 2.0;
        }
        weight_sum += 2.0;

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
                hist_match += 1.0 - (count1 - count2).abs() / count1.max(count2).max(1.0);
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
        let cfg = Cfg {
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
                defs: FxHashSet::default(),
                uses: FxHashSet::default(),
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
        let source = "def simple_func():\n    x = 1\n    y = 2\n    return x + y\n";
        let cfg = Cfg::from_source(source, "simple_func").expect("Should parse");
        assert!(!cfg.blocks.is_empty());
        let fp = cfg.fingerprint();
        assert_eq!(fp.branch_count, 0);
        assert_eq!(fp.loop_count, 0);

        // Check defs/uses
        let entry_block = &cfg.blocks[0];
        assert!(entry_block.defs.iter().any(|(n, _)| n == "x"));
        assert!(entry_block.defs.iter().any(|(n, _)| n == "y"));
        assert!(entry_block.uses.iter().any(|(n, _)| n == "x"));
        assert!(entry_block.uses.iter().any(|(n, _)| n == "y"));
    }

    #[test]
    fn test_cfg_from_source_with_if() {
        let source =
            "def func_with_if(x):\n    if x > 0:\n        return 1\n    else:\n        return -1\n";
        let cfg = Cfg::from_source(source, "func_with_if").expect("Should parse");
        let fp = cfg.fingerprint();
        assert_eq!(fp.branch_count, 1);
        assert!(fp.block_count > 1);

        // Check that 'x' usage is captured in the entry block
        assert!(cfg.blocks[0].uses.iter().any(|(n, _)| n == "x"));
    }
}
