use super::entropy::EntropyRecognizer;
use super::entropy_ast::Context;
use super::types::RawFinding;
use crate::utils::LineIndex;
use ruff_python_ast::{self as ast, ExceptHandler, StmtFunctionDef};

impl EntropyRecognizer {
    pub(super) fn visit_assign(
        &self,
        assign: &ast::StmtAssign,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        default_ctx: Context,
    ) {
        let mut safe = false;
        for target in &assign.targets {
            self.visit_expr(target, line_index, findings, default_ctx);
            if Self::check_target_safety(target) {
                safe = true;
            }
        }
        let ctx = Context {
            safe_assignment: safe,
            ..default_ctx
        };
        self.visit_expr(&assign.value, line_index, findings, ctx);
    }

    pub(super) fn visit_ann_assign(
        &self,
        assign: &ast::StmtAnnAssign,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        default_ctx: Context,
    ) {
        self.visit_expr(&assign.target, line_index, findings, default_ctx);
        let safe = Self::check_target_safety(&assign.target);
        if let Some(value) = &assign.value {
            let ctx = Context {
                safe_assignment: safe,
                ..default_ctx
            };
            self.visit_expr(value, line_index, findings, ctx);
        }
    }

    pub(super) fn visit_function(
        &self,
        function: &StmtFunctionDef,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
    ) {
        let default_ctx = Context::default();
        for decorator in &function.decorator_list {
            self.visit_expr(&decorator.expression, line_index, findings, default_ctx);
        }
        self.visit_parameter_defaults(function, line_index, findings, default_ctx);
        if let Some(return_annotation) = &function.returns {
            self.visit_expr(return_annotation, line_index, findings, default_ctx);
        }
        self.visit_stmts(&function.body, line_index, findings);
    }

    fn visit_parameter_defaults(
        &self,
        function: &StmtFunctionDef,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        default_ctx: Context,
    ) {
        for arg in &function.parameters.posonlyargs {
            if let Some(default) = &arg.default {
                self.visit_expr(default, line_index, findings, default_ctx);
            }
        }
        for arg in &function.parameters.args {
            if let Some(default) = &arg.default {
                self.visit_expr(default, line_index, findings, default_ctx);
            }
        }
        for arg in &function.parameters.kwonlyargs {
            if let Some(default) = &arg.default {
                self.visit_expr(default, line_index, findings, default_ctx);
            }
        }
    }

    pub(super) fn visit_if(
        &self,
        if_stmt: &ast::StmtIf,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        default_ctx: Context,
    ) {
        self.visit_expr(&if_stmt.test, line_index, findings, default_ctx);
        self.visit_stmts(&if_stmt.body, line_index, findings);
        for clause in &if_stmt.elif_else_clauses {
            self.visit_stmts(&clause.body, line_index, findings);
        }
    }

    pub(super) fn visit_for(
        &self,
        for_stmt: &ast::StmtFor,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        default_ctx: Context,
    ) {
        self.visit_expr(&for_stmt.target, line_index, findings, default_ctx);
        self.visit_expr(&for_stmt.iter, line_index, findings, default_ctx);
        self.visit_stmts(&for_stmt.body, line_index, findings);
        self.visit_stmts(&for_stmt.orelse, line_index, findings);
    }

    pub(super) fn visit_while(
        &self,
        while_stmt: &ast::StmtWhile,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        default_ctx: Context,
    ) {
        self.visit_expr(&while_stmt.test, line_index, findings, default_ctx);
        self.visit_stmts(&while_stmt.body, line_index, findings);
        self.visit_stmts(&while_stmt.orelse, line_index, findings);
    }

    pub(super) fn visit_try(
        &self,
        try_stmt: &ast::StmtTry,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
    ) {
        self.visit_stmts(&try_stmt.body, line_index, findings);
        for handler in &try_stmt.handlers {
            let ExceptHandler::ExceptHandler(handler) = handler;
            self.visit_stmts(&handler.body, line_index, findings);
        }
        self.visit_stmts(&try_stmt.orelse, line_index, findings);
        self.visit_stmts(&try_stmt.finalbody, line_index, findings);
    }

    pub(super) fn visit_with(
        &self,
        with_stmt: &ast::StmtWith,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        default_ctx: Context,
    ) {
        for item in &with_stmt.items {
            self.visit_expr(&item.context_expr, line_index, findings, default_ctx);
        }
        self.visit_stmts(&with_stmt.body, line_index, findings);
    }

    pub(super) fn visit_match(
        &self,
        match_stmt: &ast::StmtMatch,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        default_ctx: Context,
    ) {
        self.visit_expr(&match_stmt.subject, line_index, findings, default_ctx);
        for case in &match_stmt.cases {
            self.visit_stmts(&case.body, line_index, findings);
        }
    }
}
