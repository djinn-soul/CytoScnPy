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
            Stmt::FunctionDef(node) => self.visit_function_def(node),
            Stmt::ClassDef(node) => self.visit_body(&node.body),
            _ => self.visit_control_flow_stmt(stmt),
        }
    }

    fn visit_function_def(&mut self, node: &ast::StmtFunctionDef) {
        let mut visitor = HalsteadVisitor::new();
        visitor.add_operator(if node.is_async { "async def" } else { "def" });
        visitor.add_operand(&node.name);
        for stmt in &node.body {
            visitor.visit_stmt(stmt);
        }
        add_function_parameters(&mut visitor, node);
        self.results
            .push((node.name.to_string(), visitor.calculate_metrics()));

        self.visit_body(&node.body);
    }

    fn visit_body(&mut self, body: &[Stmt]) {
        for stmt in body {
            self.visit_stmt(stmt);
        }
    }

    fn visit_control_flow_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::If(node) => self.visit_if_stmt(node),
            Stmt::For(node) => self.visit_loop_parts(&node.body, &node.orelse),
            Stmt::While(node) => self.visit_loop_parts(&node.body, &node.orelse),
            Stmt::With(node) => self.visit_body(&node.body),
            Stmt::Match(node) => {
                for case in &node.cases {
                    self.visit_body(&case.body);
                }
            }
            Stmt::Try(node) => self.visit_try_stmt(node),
            _ => {}
        }
    }

    fn visit_if_stmt(&mut self, node: &ast::StmtIf) {
        self.visit_body(&node.body);
        for clause in &node.elif_else_clauses {
            self.visit_body(&clause.body);
        }
    }

    fn visit_loop_parts(&mut self, body: &[Stmt], orelse: &[Stmt]) {
        self.visit_body(body);
        self.visit_body(orelse);
    }

    fn visit_try_stmt(&mut self, node: &ast::StmtTry) {
        self.visit_body(&node.body);
        for handler in &node.handlers {
            let ast::ExceptHandler::ExceptHandler(handler) = handler;
            self.visit_body(&handler.body);
        }
        self.visit_body(&node.orelse);
        self.visit_body(&node.finalbody);
    }
}

fn add_function_parameters(visitor: &mut HalsteadVisitor, node: &ast::StmtFunctionDef) {
    for arg in &node.parameters.args {
        visitor.add_operand(&arg.parameter.name);
    }
    for arg in &node.parameters.posonlyargs {
        visitor.add_operand(&arg.parameter.name);
    }
    for arg in &node.parameters.kwonlyargs {
        visitor.add_operand(&arg.parameter.name);
    }
}
