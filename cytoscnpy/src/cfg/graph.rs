use crate::utils::LineIndex;
use ruff_python_ast::Stmt;
use ruff_python_parser::parse_module;
use std::collections::{HashMap, HashSet};

use super::builder::CfgBuilder;
use super::types::{Cfg, CfgFingerprint, StmtKind};

impl Cfg {
    /// Constructs a CFG from a function AST node and its line index.
    pub fn from_function(func: &ruff_python_ast::StmtFunctionDef, line_index: &LineIndex) -> Self {
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

    /// Identifies all basic blocks that are not reachable from the entry point.
    #[must_use]
    pub fn find_unreachable_blocks(&self) -> Vec<usize> {
        let mut reachable = vec![false; self.blocks.len()];
        let mut stack = vec![self.entry];

        while let Some(block_id) = stack.pop() {
            if block_id >= reachable.len() || reachable[block_id] {
                continue;
            }
            reachable[block_id] = true;
            for &successor in &self.blocks[block_id].successors {
                stack.push(successor);
            }
        }

        self.blocks
            .iter()
            .enumerate()
            .filter(|(id, _)| !reachable[*id])
            .map(|(id, _)| id)
            .collect()
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

        let all_kinds: HashSet<_> = fp1
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
