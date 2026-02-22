mod visits;

use crate::utils::LineIndex;
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::Ranged;

use super::collector::NameCollector;
use super::types::{BasicBlock, Cfg, StmtKind, StmtRef};

/// Builder for constructing CFG from AST.
pub(super) struct CfgBuilder<'a> {
    blocks: Vec<BasicBlock>,
    current_block: usize,
    loop_depth: usize,
    /// Stack of (`loop_header_id`, `loop_exit_id`) for break/continue.
    loop_stack: Vec<(usize, usize)>,
    line_index: &'a LineIndex,
}

impl<'a> CfgBuilder<'a> {
    pub(super) fn new(line_index: &'a LineIndex) -> Self {
        let entry_block = BasicBlock::new(0, 0);
        Self {
            blocks: vec![entry_block],
            current_block: 0,
            loop_depth: 0,
            loop_stack: Vec::new(),
            line_index,
        }
    }

    pub(super) fn new_block(&mut self) -> usize {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock::new(id, self.loop_depth));
        id
    }

    pub(super) fn add_edge(&mut self, from: usize, to: usize) {
        if !self.blocks[from].successors.contains(&to) {
            self.blocks[from].successors.push(to);
        }
        if !self.blocks[to].predecessors.contains(&from) {
            self.blocks[to].predecessors.push(from);
        }
    }

    pub(super) fn add_stmt(&mut self, kind: StmtKind, line: usize) {
        self.blocks[self.current_block]
            .statements
            .push(StmtRef { line, kind });
    }

    pub(super) fn build_from_function(&mut self, func: &ast::StmtFunctionDef) {
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

    pub(super) fn build_from_body(&mut self, body: &[Stmt]) {
        for stmt in body {
            self.visit_stmt(stmt);
        }
    }

    pub(super) fn collect_expr_names(&mut self, expr: &ast::Expr, line: usize) {
        let block = &mut self.blocks[self.current_block];
        let mut collector = NameCollector {
            defs: &mut block.defs,
            uses: &mut block.uses,
            current_line: line,
        };
        collector.visit_expr(expr);
    }

    pub(super) fn collect_pattern_names(&mut self, pattern: &ast::Pattern, line: usize) {
        let block = &mut self.blocks[self.current_block];
        let mut collector = NameCollector {
            defs: &mut block.defs,
            uses: &mut block.uses,
            current_line: line,
        };
        collector.visit_pattern(pattern);
    }

    pub(super) fn collect_stmt_names(&mut self, stmt: &Stmt, line: usize) {
        let block = &mut self.blocks[self.current_block];
        let mut collector = NameCollector {
            defs: &mut block.defs,
            uses: &mut block.uses,
            current_line: line,
        };
        collector.visit_stmt(stmt);
    }

    pub(super) fn build(self) -> Cfg {
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
