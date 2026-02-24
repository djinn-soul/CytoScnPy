#![allow(missing_docs)]

use super::*;

impl<'a> CytoScnPyVisitor<'a> {
    pub fn visit_stmt(&mut self, stmt: &Stmt) {
        if self.depth >= MAX_RECURSION_DEPTH {
            self.recursion_limit_hit = true;
            return;
        }
        self.depth += 1;

        match stmt {
            Stmt::FunctionDef(node) => self.handle_function_stmt(node),
            Stmt::ClassDef(node) => self.handle_class_stmt(node),
            Stmt::Import(node) => self.handle_import_stmt(node),
            Stmt::ImportFrom(node) => self.handle_import_from_stmt(node),
            Stmt::Assign(node) => self.handle_assign_stmt(node),
            Stmt::AugAssign(node) => {
                self.visit_expr(&node.target);
                self.visit_expr(&node.value);
            }
            Stmt::AnnAssign(node) => self.handle_ann_assign_stmt(node),
            Stmt::Expr(node) => self.visit_expr(&node.value),
            Stmt::If(node) => self.handle_if_stmt(node),
            Stmt::For(node) => self.handle_for_stmt(node),
            Stmt::While(node) => self.handle_while_stmt(node),
            Stmt::With(node) => self.handle_with_stmt(node),
            Stmt::Try(node) => self.handle_try_stmt(node),
            Stmt::Return(node) => self.handle_return_stmt(node),
            Stmt::Assert(node) => self.handle_assert_stmt(node),
            Stmt::Raise(node) => self.handle_raise_stmt(node),
            Stmt::Delete(node) => self.handle_delete_stmt(node),
            Stmt::Match(node) => self.handle_match_stmt(node),
            Stmt::Global(node) => self.handle_global_stmt(node),
            _ => {}
        }

        self.depth -= 1;
    }
}
