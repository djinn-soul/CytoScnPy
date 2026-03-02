use crate::metrics::cc_rank;
use crate::utils::LineIndex;
use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::Ranged;

use super::analysis::ComplexityFinding;
use super::block::calculate_complexity;

pub(super) struct ComplexityVisitor<'a> {
    pub(super) findings: Vec<ComplexityFinding>,
    line_index: &'a LineIndex,
    class_stack: Vec<String>,
    no_assert: bool,
}

impl<'a> ComplexityVisitor<'a> {
    pub(super) fn new(line_index: &'a LineIndex, no_assert: bool) -> Self {
        Self {
            findings: Vec::new(),
            line_index,
            class_stack: Vec::new(),
            no_assert,
        }
    }

    pub(super) fn visit_body(&mut self, body: &[Stmt]) {
        for stmt in body {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::FunctionDef(node) => {
                let complexity = calculate_complexity(&node.body, self.no_assert);
                let rank = cc_rank(complexity);
                let line = self.line_index.line_index(node.start());
                let type_ = if self.class_stack.is_empty() {
                    "function"
                } else {
                    "method"
                };

                self.findings.push(ComplexityFinding {
                    name: node.name.to_string(),
                    complexity,
                    rank,
                    type_: type_.to_owned(),
                    line,
                });

                self.visit_body(&node.body);
            }
            Stmt::ClassDef(node) => {
                let complexity = calculate_complexity(&node.body, self.no_assert);
                let rank = cc_rank(complexity);
                let line = self.line_index.line_index(node.start());

                self.findings.push(ComplexityFinding {
                    name: node.name.to_string(),
                    complexity,
                    rank,
                    type_: "class".to_owned(),
                    line,
                });

                self.class_stack.push(node.name.to_string());
                self.visit_body(&node.body);
                self.class_stack.pop();
            }
            _ => self.visit_nested_defs(stmt),
        }
    }

    fn visit_nested_defs(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::If(node) => {
                self.visit_body(&node.body);
                for clause in &node.elif_else_clauses {
                    self.visit_body(&clause.body);
                }
            }
            Stmt::For(node) => {
                self.visit_body(&node.body);
                self.visit_body(&node.orelse);
            }
            Stmt::While(node) => {
                self.visit_body(&node.body);
                self.visit_body(&node.orelse);
            }
            Stmt::With(node) => {
                self.visit_body(&node.body);
            }
            Stmt::Try(node) => {
                self.visit_body(&node.body);
                for handler in &node.handlers {
                    let ast::ExceptHandler::ExceptHandler(except_handler) = handler;
                    self.visit_body(&except_handler.body);
                }
                self.visit_body(&node.finalbody);
                self.visit_body(&node.orelse);
            }
            Stmt::Match(node) => {
                for case in &node.cases {
                    self.visit_body(&case.body);
                }
            }
            _ => {}
        }
    }
}
