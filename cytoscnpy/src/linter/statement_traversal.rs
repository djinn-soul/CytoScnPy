use super::LinterVisitor;
use ruff_python_ast::{Expr, Stmt};

impl LinterVisitor {
    /// Visits a statement node and applies rules.
    pub fn visit_stmt(&mut self, stmt: &Stmt) {
        self.apply_enter_stmt_rules(stmt);
        self.visit_stmt_children(stmt);
        self.apply_leave_stmt_rules(stmt);
    }

    fn apply_enter_stmt_rules(&mut self, stmt: &Stmt) {
        for rule in &mut self.rules {
            if let Some(mut findings) = rule.enter_stmt(stmt, &self.context) {
                self.findings.append(&mut findings);
            }
        }
    }

    fn apply_leave_stmt_rules(&mut self, stmt: &Stmt) {
        for rule in &mut self.rules {
            if let Some(mut findings) = rule.leave_stmt(stmt, &self.context) {
                self.findings.append(&mut findings);
            }
        }
    }

    fn visit_stmt_children(&mut self, stmt: &Stmt) {
        if self.visit_definition_stmt_children(stmt) {
            return;
        }
        if self.visit_branch_stmt_children(stmt) {
            return;
        }
        if self.visit_try_with_stmt_children(stmt) {
            return;
        }
        if self.visit_value_stmt_children(stmt) {
            return;
        }
        let _ = self.visit_optional_value_stmt_children(stmt);
    }

    fn visit_definition_stmt_children(&mut self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::FunctionDef(node) => self.visit_stmts(&node.body),
            Stmt::ClassDef(node) => self.visit_stmts(&node.body),
            _ => return false,
        }
        true
    }

    fn visit_branch_stmt_children(&mut self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::If(node) => {
                self.visit_expr(&node.test);
                self.visit_stmts(&node.body);
                for clause in &node.elif_else_clauses {
                    self.visit_stmts(&clause.body);
                }
            }
            Stmt::For(node) => {
                self.visit_expr(&node.iter);
                self.visit_stmts(&node.body);
                self.visit_stmts(&node.orelse);
            }
            Stmt::While(node) => {
                self.visit_expr(&node.test);
                self.visit_stmts(&node.body);
                self.visit_stmts(&node.orelse);
            }
            _ => return false,
        }
        true
    }

    fn visit_try_with_stmt_children(&mut self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Try(node) => {
                self.visit_stmts(&node.body);
                for handler in &node.handlers {
                    match handler {
                        ruff_python_ast::ExceptHandler::ExceptHandler(h) => {
                            self.visit_stmts(&h.body);
                        }
                    }
                }
                self.visit_stmts(&node.orelse);
                self.visit_stmts(&node.finalbody);
            }
            Stmt::With(node) => self.visit_stmts(&node.body),
            _ => return false,
        }
        true
    }

    fn visit_value_stmt_children(&mut self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Expr(node) => self.visit_expr(&node.value),
            Stmt::Assign(node) => self.visit_expr(&node.value),
            Stmt::AugAssign(node) => self.visit_expr(&node.value),
            _ => return false,
        }
        true
    }

    fn visit_optional_value_stmt_children(&mut self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::AnnAssign(node) => self.visit_optional_expr(node.value.as_deref()),
            Stmt::Return(node) => self.visit_optional_expr(node.value.as_deref()),
            _ => return false,
        }
        true
    }

    fn visit_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.visit_stmt(stmt);
        }
    }

    pub(super) fn visit_optional_expr(&mut self, expr: Option<&Expr>) {
        if let Some(expr) = expr {
            self.visit_expr(expr);
        }
    }
}
