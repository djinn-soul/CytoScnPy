use super::entropy::EntropyRecognizer;
use super::entropy_ast::Context;
use super::types::RawFinding;
use crate::utils::LineIndex;
use ruff_python_ast::Stmt;

impl EntropyRecognizer {
    pub(super) fn visit_stmts(
        &self,
        stmts: &[Stmt],
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
    ) {
        for stmt in stmts {
            self.visit_stmt(stmt, line_index, findings);
        }
    }

    fn visit_stmt(&self, stmt: &Stmt, line_index: &LineIndex, findings: &mut Vec<RawFinding>) {
        let default_ctx = Context::default();
        if self.visit_assignment_stmt(stmt, line_index, findings, default_ctx) {
            return;
        }
        if self.visit_simple_stmt(stmt, line_index, findings, default_ctx) {
            return;
        }
        self.visit_compound_stmt(stmt, line_index, findings, default_ctx);
    }

    fn visit_assignment_stmt(
        &self,
        stmt: &Stmt,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        default_ctx: Context,
    ) -> bool {
        match stmt {
            Stmt::Assign(assign) => {
                self.visit_assign(assign, line_index, findings, default_ctx);
                true
            }
            Stmt::AnnAssign(assign) => {
                self.visit_ann_assign(assign, line_index, findings, default_ctx);
                true
            }
            _ => false,
        }
    }

    fn visit_simple_stmt(
        &self,
        stmt: &Stmt,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        default_ctx: Context,
    ) -> bool {
        self.visit_expr_or_return_stmt(stmt, line_index, findings, default_ctx)
            || self.visit_mutation_stmt(stmt, line_index, findings, default_ctx)
            || self.visit_exception_stmt(stmt, line_index, findings, default_ctx)
    }

    fn visit_expr_or_return_stmt(
        &self,
        stmt: &Stmt,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        default_ctx: Context,
    ) -> bool {
        match stmt {
            Stmt::Expr(expr) => self.visit_expr(&expr.value, line_index, findings, default_ctx),
            Stmt::Return(ret) => {
                if let Some(value) = ret.value.as_deref() {
                    self.visit_expr(value, line_index, findings, default_ctx);
                }
            }
            _ => return false,
        }
        true
    }

    fn visit_mutation_stmt(
        &self,
        stmt: &Stmt,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        default_ctx: Context,
    ) -> bool {
        match stmt {
            Stmt::AugAssign(assign) => {
                self.visit_expr(&assign.target, line_index, findings, default_ctx);
                self.visit_expr(&assign.value, line_index, findings, default_ctx);
            }
            Stmt::Delete(delete) => {
                for target in &delete.targets {
                    self.visit_expr(target, line_index, findings, default_ctx);
                }
            }
            _ => return false,
        }
        true
    }

    fn visit_exception_stmt(
        &self,
        stmt: &Stmt,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        default_ctx: Context,
    ) -> bool {
        match stmt {
            Stmt::Raise(raise) => {
                if let Some(exc) = &raise.exc {
                    self.visit_expr(exc, line_index, findings, default_ctx);
                }
                if let Some(cause) = &raise.cause {
                    self.visit_expr(cause, line_index, findings, default_ctx);
                }
            }
            Stmt::Assert(assert) => {
                self.visit_expr(&assert.test, line_index, findings, default_ctx);
                if let Some(msg) = &assert.msg {
                    self.visit_expr(msg, line_index, findings, default_ctx);
                }
            }
            _ => return false,
        }
        true
    }

    fn visit_compound_stmt(
        &self,
        stmt: &Stmt,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        default_ctx: Context,
    ) {
        if self.visit_definition_stmt(stmt, line_index, findings) {
            return;
        }
        if self.visit_branch_stmt(stmt, line_index, findings, default_ctx) {
            return;
        }
        self.visit_context_stmt(stmt, line_index, findings, default_ctx);
    }

    fn visit_definition_stmt(
        &self,
        stmt: &Stmt,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
    ) -> bool {
        match stmt {
            Stmt::FunctionDef(function) => self.visit_function(function, line_index, findings),
            Stmt::ClassDef(class) => self.visit_stmts(&class.body, line_index, findings),
            _ => return false,
        }
        true
    }

    fn visit_branch_stmt(
        &self,
        stmt: &Stmt,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        default_ctx: Context,
    ) -> bool {
        match stmt {
            Stmt::If(if_stmt) => self.visit_if(if_stmt, line_index, findings, default_ctx),
            Stmt::For(for_stmt) => self.visit_for(for_stmt, line_index, findings, default_ctx),
            Stmt::While(while_stmt) => {
                self.visit_while(while_stmt, line_index, findings, default_ctx);
            }
            Stmt::Try(try_stmt) => self.visit_try(try_stmt, line_index, findings),
            _ => return false,
        }
        true
    }

    fn visit_context_stmt(
        &self,
        stmt: &Stmt,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        default_ctx: Context,
    ) {
        match stmt {
            Stmt::With(with_stmt) => self.visit_with(with_stmt, line_index, findings, default_ctx),
            Stmt::Match(match_stmt) => {
                self.visit_match(match_stmt, line_index, findings, default_ctx);
            }
            _ => {}
        }
    }
}
