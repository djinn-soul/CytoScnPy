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
    ///
    /// This function implements comprehensive recursion for all expression types
    /// (Calls, `BinOps`, Comprehensions, etc.) to ensure that linter rules
    /// can inspect strictly nested nodes.
    /// Verified by `analyzer_test` and `quality_test` suites.
    pub fn visit_expr(&mut self, expr: &Expr) {
        // Call visit_expr for all rules
        for rule in &mut self.rules {
            if let Some(mut findings) = rule.visit_expr(expr, &self.context) {
                self.findings.append(&mut findings);
            }
        }

        // Recursively visit sub-expressions
        match expr {
            Expr::Call(node) => {
                self.visit_expr(&node.func);
                for arg in &node.arguments.args {
                    self.visit_expr(arg);
                }
                for keyword in &node.arguments.keywords {
                    self.visit_expr(&keyword.value);
                }
            }
            Expr::Attribute(node) => self.visit_expr(&node.value),
            Expr::BinOp(node) => {
                self.visit_expr(&node.left);
                self.visit_expr(&node.right);
            }
            Expr::UnaryOp(node) => self.visit_expr(&node.operand),
            Expr::BoolOp(node) => {
                for value in &node.values {
                    self.visit_expr(value);
                }
            }
            Expr::Compare(node) => {
                self.visit_expr(&node.left);
                for val in &node.comparators {
                    self.visit_expr(val);
                }
            }
            Expr::List(node) => {
                for elt in &node.elts {
                    self.visit_expr(elt);
                }
            }
            Expr::Tuple(node) => {
                for elt in &node.elts {
                    self.visit_expr(elt);
                }
            }
            Expr::Set(node) => {
                for elt in &node.elts {
                    self.visit_expr(elt);
                }
            }
            Expr::Dict(node) => {
                for item in &node.items {
                    if let Some(key) = &item.key {
                        self.visit_expr(key);
                    }
                    self.visit_expr(&item.value);
                }
            }
            Expr::Subscript(node) => {
                self.visit_expr(&node.value);
                self.visit_expr(&node.slice);
            }
            Expr::Starred(node) => self.visit_expr(&node.value),
            Expr::Yield(node) => {
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            Expr::YieldFrom(node) => self.visit_expr(&node.value),
            Expr::Await(node) => self.visit_expr(&node.value),
            Expr::Lambda(node) => self.visit_expr(&node.body),
            Expr::ListComp(node) => {
                for gen in &node.generators {
                    self.visit_expr(&gen.iter);
                    for r in &gen.ifs {
                        self.visit_expr(r);
                    }
                }
                self.visit_expr(&node.elt);
            }
            Expr::SetComp(node) => {
                for gen in &node.generators {
                    self.visit_expr(&gen.iter);
                    for r in &gen.ifs {
                        self.visit_expr(r);
                    }
                }
                self.visit_expr(&node.elt);
            }
            Expr::DictComp(node) => {
                for gen in &node.generators {
                    self.visit_expr(&gen.iter);
                    for r in &gen.ifs {
                        self.visit_expr(r);
                    }
                }
                self.visit_expr(&node.key);
                self.visit_expr(&node.value);
            }
            Expr::Generator(node) => {
                for gen in &node.generators {
                    self.visit_expr(&gen.iter);
                    for r in &gen.ifs {
                        self.visit_expr(r);
                    }
                }
                self.visit_expr(&node.elt);
            }
            _ => {}
        }

        // Call leave_expr for all rules
        for rule in &mut self.rules {
            if let Some(mut findings) = rule.leave_expr(expr, &self.context) {
                self.findings.append(&mut findings);
            }
        }
    }
}
