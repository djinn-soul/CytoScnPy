use crate::config::Config;
use crate::rules::{Context, Finding, Rule};
use crate::utils::LineIndex;
use ruff_python_ast::{Expr, Stmt};
use std::path::PathBuf;

/// Visitor for traversing the AST and applying linter rules.
pub struct LinterVisitor {
    rules: Vec<Box<dyn Rule>>,
    context: Context,
    /// List of findings collected during the traversal.
    pub findings: Vec<Finding>,
}

impl LinterVisitor {
    /// Creates a new `LinterVisitor` with the given rules and context.
    #[must_use]
    pub fn new(
        rules: Vec<Box<dyn Rule>>,
        filename: PathBuf,
        line_index: LineIndex,
        config: Config,
    ) -> Self {
        Self {
            rules,
            context: Context {
                filename,
                line_index,
                config,
            },
            findings: Vec::new(),
        }
    }

    /// Visits a statement node and applies rules.
    pub fn visit_stmt(&mut self, stmt: &Stmt) {
        // Call enter_stmt for all rules
        for rule in &mut self.rules {
            if let Some(mut findings) = rule.enter_stmt(stmt, &self.context) {
                self.findings.append(&mut findings);
            }
        }

        // Manually walk children
        match stmt {
            Stmt::FunctionDef(node) => {
                for s in &node.body {
                    self.visit_stmt(s);
                }
            }
            Stmt::ClassDef(node) => {
                for s in &node.body {
                    self.visit_stmt(s);
                }
            }
            Stmt::If(node) => {
                self.visit_expr(&node.test);
                for s in &node.body {
                    self.visit_stmt(s);
                }
                for clause in &node.elif_else_clauses {
                    self.visit_stmt(&clause.body[0]); // Hack: elif_else_clauses body is Vec<Stmt>, but visitor expects single Stmt recursion? No, loop over them.
                    for s in &clause.body {
                        self.visit_stmt(s);
                    }
                }
            }
            Stmt::For(node) => {
                self.visit_expr(&node.iter);
                for s in &node.body {
                    self.visit_stmt(s);
                }
                for s in &node.orelse {
                    self.visit_stmt(s);
                }
            }
            Stmt::While(node) => {
                self.visit_expr(&node.test);
                for s in &node.body {
                    self.visit_stmt(s);
                }
                for s in &node.orelse {
                    self.visit_stmt(s);
                }
            }
            Stmt::Try(node) => {
                for s in &node.body {
                    self.visit_stmt(s);
                }
                for handler in &node.handlers {
                    match handler {
                        ruff_python_ast::ExceptHandler::ExceptHandler(h) => {
                            for s in &h.body {
                                self.visit_stmt(s);
                            }
                        }
                    }
                }
                for s in &node.orelse {
                    self.visit_stmt(s);
                }
                for s in &node.finalbody {
                    self.visit_stmt(s);
                }
            }
            Stmt::With(node) => {
                for s in &node.body {
                    self.visit_stmt(s);
                }
            }
            Stmt::Expr(node) => {
                self.visit_expr(&node.value);
            }
            Stmt::Assign(node) => {
                self.visit_expr(&node.value);
            }
            Stmt::AnnAssign(node) => {
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            Stmt::AugAssign(node) => {
                self.visit_expr(&node.value);
            }
            Stmt::Return(node) => {
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            _ => {}
        }

        // Call leave_stmt for all rules
        for rule in &mut self.rules {
            if let Some(mut findings) = rule.leave_stmt(stmt, &self.context) {
                self.findings.append(&mut findings);
            }
        }
    }

    /// Visits an expression node and applies rules.
    pub fn visit_expr(&mut self, expr: &Expr) {
        // Call visit_expr for all rules
        for rule in &mut self.rules {
            if let Some(mut findings) = rule.visit_expr(expr, &self.context) {
                self.findings.append(&mut findings);
            }
        }

        // Recursively visit sub-expressions if needed
        // For now, rules check what they need
    }
}
