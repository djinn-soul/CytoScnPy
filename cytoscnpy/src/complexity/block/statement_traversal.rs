use super::{is_wildcard_case, BlockComplexityVisitor};
use ruff_python_ast::{self as ast, Stmt};

impl BlockComplexityVisitor {
    pub(super) fn visit_stmt(&mut self, stmt: &Stmt) {
        if self.visit_branch_stmt(stmt) {
            return;
        }
        if self.visit_loop_stmt(stmt) {
            return;
        }
        if self.visit_context_stmt(stmt) {
            return;
        }
        if self.visit_definition_stmt(stmt) {
            return;
        }
        if self.visit_assignment_stmt(stmt) {
            return;
        }
        let _ = self.visit_simple_stmt(stmt);
    }

    fn visit_branch_stmt(&mut self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::If(node) => {
                self.complexity += 1;
                self.visit_expr(&node.test);
                self.visit_body(&node.body);
                for clause in &node.elif_else_clauses {
                    if let Some(test) = &clause.test {
                        self.complexity += 1;
                        self.visit_expr(test);
                    }
                    self.visit_body(&clause.body);
                }
            }
            Stmt::Try(node) => self.visit_try_stmt(node),
            Stmt::Match(node) => {
                self.visit_expr(&node.subject);
                for case in &node.cases {
                    if !is_wildcard_case(&case.pattern) {
                        self.complexity += 1;
                    }
                    if let Some(guard) = &case.guard {
                        self.visit_expr(guard);
                    }
                    self.visit_body(&case.body);
                }
            }
            _ => return false,
        }
        true
    }

    fn visit_try_stmt(&mut self, node: &ast::StmtTry) {
        self.visit_body(&node.body);
        for handler in &node.handlers {
            self.complexity += 1;
            let ast::ExceptHandler::ExceptHandler(except_handler) = handler;
            if let Some(type_) = &except_handler.type_ {
                self.visit_expr(type_);
            }
            self.visit_body(&except_handler.body);
        }
        if !node.orelse.is_empty() {
            self.complexity += 1;
        }
        self.visit_body(&node.orelse);
        self.visit_body(&node.finalbody);
    }

    fn visit_loop_stmt(&mut self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::For(node) => {
                self.complexity += 1;
                self.visit_expr(&node.target);
                self.visit_expr(&node.iter);
                self.visit_body(&node.body);
                self.visit_optional_else(&node.orelse);
            }
            Stmt::While(node) => {
                self.complexity += 1;
                self.visit_expr(&node.test);
                self.visit_body(&node.body);
                self.visit_optional_else(&node.orelse);
            }
            _ => return false,
        }
        true
    }

    fn visit_optional_else(&mut self, body: &[Stmt]) {
        if !body.is_empty() {
            self.complexity += 1;
        }
        self.visit_body(body);
    }

    fn visit_context_stmt(&mut self, stmt: &Stmt) -> bool {
        let Stmt::With(node) = stmt else {
            return false;
        };

        for item in &node.items {
            self.visit_expr(&item.context_expr);
            if let Some(optional_vars) = &item.optional_vars {
                self.visit_expr(optional_vars);
            }
        }
        self.visit_body(&node.body);
        true
    }

    fn visit_definition_stmt(&mut self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::FunctionDef(node) if self.descend_definitions => self.visit_body(&node.body),
            Stmt::ClassDef(node) if self.descend_definitions => self.visit_body(&node.body),
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            _ => return false,
        }
        true
    }

    fn visit_assignment_stmt(&mut self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Assign(node) => {
                for target in &node.targets {
                    self.visit_expr(target);
                }
                self.visit_expr(&node.value);
            }
            Stmt::AugAssign(node) => {
                self.visit_expr(&node.target);
                self.visit_expr(&node.value);
            }
            Stmt::AnnAssign(node) => {
                self.visit_expr(&node.target);
                self.visit_expr(&node.annotation);
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            Stmt::Delete(node) => {
                for target in &node.targets {
                    self.visit_expr(target);
                }
            }
            _ => return false,
        }
        true
    }

    fn visit_simple_stmt(&mut self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Expr(node) => self.visit_expr(&node.value),
            Stmt::Return(node) => {
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            Stmt::Assert(node) => {
                if !self.no_assert {
                    self.complexity += 1;
                }
                self.visit_expr(&node.test);
                if let Some(msg) = &node.msg {
                    self.visit_expr(msg);
                }
            }
            Stmt::Raise(node) => {
                if let Some(exc) = &node.exc {
                    self.visit_expr(exc);
                }
                if let Some(cause) = &node.cause {
                    self.visit_expr(cause);
                }
            }
            _ => return false,
        }
        true
    }
}
