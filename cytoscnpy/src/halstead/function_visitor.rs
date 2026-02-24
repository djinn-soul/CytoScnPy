use ruff_python_ast::{self as ast, Stmt};

use super::visitor::HalsteadVisitor;
use super::HalsteadMetrics;

pub(super) struct FunctionHalsteadVisitor {
    pub(super) results: Vec<(String, HalsteadMetrics)>,
}

impl FunctionHalsteadVisitor {
    pub(super) fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    pub(super) fn visit_mod(&mut self, module: &ast::Mod) {
        if let ast::Mod::Module(m) = module {
            for stmt in &m.body {
                self.visit_stmt(stmt);
            }
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::FunctionDef(node) => {
                let mut visitor = HalsteadVisitor::new();
                if node.is_async {
                    visitor.add_operator("async def");
                }
                for s in &node.body {
                    visitor.visit_stmt(s);
                }
                for arg in &node.parameters.args {
                    visitor.add_operand(&arg.parameter.name);
                }
                for arg in &node.parameters.posonlyargs {
                    visitor.add_operand(&arg.parameter.name);
                }
                for arg in &node.parameters.kwonlyargs {
                    visitor.add_operand(&arg.parameter.name);
                }
                self.results
                    .push((node.name.to_string(), visitor.calculate_metrics()));

                for s in &node.body {
                    self.visit_stmt(s);
                }
            }
            Stmt::ClassDef(node) => {
                for s in &node.body {
                    self.visit_stmt(s);
                }
            }
            _ => match stmt {
                Stmt::If(node) => {
                    for s in &node.body {
                        self.visit_stmt(s);
                    }
                    for clause in &node.elif_else_clauses {
                        self.visit_stmt(&clause.body[0]);
                        for s in &clause.body {
                            self.visit_stmt(s);
                        }
                    }
                }
                Stmt::For(node) => {
                    for s in &node.body {
                        self.visit_stmt(s);
                    }
                    for s in &node.orelse {
                        self.visit_stmt(s);
                    }
                }
                Stmt::While(node) => {
                    for s in &node.body {
                        self.visit_stmt(s);
                    }
                    for s in &node.orelse {
                        self.visit_stmt(s);
                    }
                }
                Stmt::With(node) => {
                    for s in &node.body {
                        self.visit_stmt(s);
                    }
                }
                Stmt::Try(node) => {
                    for s in &node.body {
                        self.visit_stmt(s);
                    }
                    for handler in &node.handlers {
                        let ast::ExceptHandler::ExceptHandler(h) = handler;
                        for s in &h.body {
                            self.visit_stmt(s);
                        }
                    }
                    for s in &node.orelse {
                        self.visit_stmt(s);
                    }
                    for s in &node.finalbody {
                        self.visit_stmt(s);
                    }
                }
                _ => {}
            },
        }
    }
}
