mod expressions;
mod statements;

use ruff_python_ast::{self as ast, Expr, Stmt};
use rustc_hash::FxHashSet;

use super::metrics::HalsteadMetrics;

pub(super) struct HalsteadVisitor {
    operators: FxHashSet<String>,
    operands: FxHashSet<String>,
    total_operators: usize,
    total_operands: usize,
}

impl HalsteadVisitor {
    pub(crate) fn new() -> Self {
        Self {
            operators: FxHashSet::default(),
            operands: FxHashSet::default(),
            total_operators: 0,
            total_operands: 0,
        }
    }

    pub(crate) fn add_operator(&mut self, op: &str) {
        self.operators.insert(op.to_owned());
        self.total_operators += 1;
    }

    pub(crate) fn add_operand(&mut self, op: &str) {
        self.operands.insert(op.to_owned());
        self.total_operands += 1;
    }

    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn calculate_metrics(&self) -> HalsteadMetrics {
        let n1 = self.operators.len() as f64;
        let n2 = self.operands.len() as f64;
        let n1_total = self.total_operators as f64;
        let n2_total = self.total_operands as f64;

        let vocabulary = n1 + n2;
        let length = n1_total + n2_total;
        let calculated_length = if n1 > 0.0 && n2 > 0.0 {
            n1 * n1.log2() + n2 * n2.log2()
        } else {
            0.0
        };
        let volume = if vocabulary > 0.0 {
            length * vocabulary.log2()
        } else {
            0.0
        };
        let difficulty = if n2 > 0.0 {
            (n1 / 2.0) * (n2_total / n2)
        } else {
            0.0
        };
        let effort = difficulty * volume;
        let time = effort / 18.0;
        let bugs = volume / 3000.0;

        HalsteadMetrics {
            h1: self.total_operators,
            h2: self.total_operands,
            n1: self.operators.len(),
            n2: self.operands.len(),
            vocabulary,
            length,
            calculated_length,
            volume,
            difficulty,
            effort,
            time,
            bugs,
        }
    }

    pub(crate) fn visit_mod(&mut self, module: &ast::Mod) {
        if let ast::Mod::Module(m) = module {
            for stmt in &m.body {
                self.visit_stmt(stmt);
            }
        }
    }

    pub(crate) fn visit_stmt(&mut self, stmt: &Stmt) {
        statements::visit_stmt(self, stmt);
    }

    pub(crate) fn visit_expr(&mut self, expr: &Expr) {
        expressions::visit_expr(self, expr);
    }
}
