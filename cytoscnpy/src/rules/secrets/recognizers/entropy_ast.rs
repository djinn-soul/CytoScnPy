use super::entropy::EntropyRecognizer;
use super::types::{is_test_name, RawFinding};
use crate::utils::LineIndex;
use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

/// Context for AST traversal.
#[derive(Clone, Copy, Default)]
pub(super) struct Context {
    pub(super) in_logging: bool,
    pub(super) safe_assignment: bool,
}

impl EntropyRecognizer {
    /// Check if an expression is a call to logging/print functions.
    fn is_logging_call(expr: &Expr) -> bool {
        if let Expr::Call(call) = expr {
            match &*call.func {
                Expr::Attribute(attr) => {
                    let attr_name = attr.attr.as_str();
                    // Common logging methods
                    if matches!(
                        attr_name,
                        "debug"
                            | "info"
                            | "warning"
                            | "warn"
                            | "error"
                            | "critical"
                            | "log"
                            | "exception"
                    ) {
                        // Check if base is 'logger' or 'logging' or 'log' or 'self.logger'
                        if let Expr::Name(name) = &*attr.value {
                            let id = name.id.as_str();
                            return matches!(id, "logger" | "logging" | "log");
                        }
                        // Handle self.logger.info
                        if let Expr::Attribute(inner_attr) = &*attr.value {
                            if inner_attr.attr.as_str() == "logger" {
                                return true;
                            }
                        }
                    }
                    false
                }
                Expr::Name(name) => {
                    // check for global 'print' or 'log'
                    matches!(name.id.as_str(), "print" | "log")
                }
                _ => false,
            }
        } else {
            false
        }
    }

    pub(super) fn visit_expr(
        &self,
        expr: &Expr,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        ctx: Context,
    ) {
        match expr {
            Expr::StringLiteral(s) if !ctx.in_logging && !ctx.safe_assignment => {
                self.check_string(
                    s.value.to_str(),
                    line_index.line_index(expr.start()),
                    findings,
                );
            }
            Expr::Call(call) => self.visit_call_expr(call, expr, line_index, findings, ctx),
            Expr::List(list) => self.visit_exprs(&list.elts, line_index, findings, ctx),
            Expr::Tuple(tuple) => self.visit_exprs(&tuple.elts, line_index, findings, ctx),
            Expr::Set(set) => self.visit_exprs(&set.elts, line_index, findings, ctx),
            Expr::Dict(dict) => self.visit_dict_expr(dict, line_index, findings, ctx),
            Expr::BinOp(b) => {
                self.visit_expr(&b.left, line_index, findings, ctx);
                self.visit_expr(&b.right, line_index, findings, ctx);
            }
            Expr::UnaryOp(u) => {
                self.visit_expr(&u.operand, line_index, findings, ctx);
            }
            Expr::BoolOp(bool_op) => self.visit_exprs(&bool_op.values, line_index, findings, ctx),
            Expr::Attribute(a) => {
                self.visit_expr(&a.value, line_index, findings, ctx);
            }
            Expr::Subscript(s) => {
                self.visit_expr(&s.value, line_index, findings, ctx);
                self.visit_expr(&s.slice, line_index, findings, ctx);
            }
            Expr::Await(a) => self.visit_expr(&a.value, line_index, findings, ctx),
            Expr::Yield(y) => {
                self.visit_optional_expr(y.value.as_deref(), line_index, findings, ctx);
            }
            Expr::YieldFrom(y) => self.visit_expr(&y.value, line_index, findings, ctx),
            Expr::Compare(compare) => self.visit_compare_expr(compare, line_index, findings, ctx),
            _ => {}
        }
    }

    fn visit_call_expr(
        &self,
        call: &ruff_python_ast::ExprCall,
        expr: &Expr,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        ctx: Context,
    ) {
        let new_ctx = Context {
            in_logging: ctx.in_logging || Self::is_logging_call(expr),
            ..ctx
        };
        self.visit_expr(&call.func, line_index, findings, ctx);
        self.visit_exprs(&call.arguments.args, line_index, findings, new_ctx);
        for keyword in &call.arguments.keywords {
            self.visit_expr(&keyword.value, line_index, findings, new_ctx);
        }
    }

    fn visit_dict_expr(
        &self,
        dict: &ruff_python_ast::ExprDict,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        ctx: Context,
    ) {
        for item in &dict.items {
            self.visit_optional_expr(item.key.as_ref(), line_index, findings, ctx);
            self.visit_expr(&item.value, line_index, findings, ctx);
        }
    }

    fn visit_compare_expr(
        &self,
        compare: &ruff_python_ast::ExprCompare,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        ctx: Context,
    ) {
        self.visit_expr(&compare.left, line_index, findings, ctx);
        self.visit_exprs(&compare.comparators, line_index, findings, ctx);
    }

    fn visit_optional_expr(
        &self,
        expr: Option<&Expr>,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        ctx: Context,
    ) {
        if let Some(expr) = expr {
            self.visit_expr(expr, line_index, findings, ctx);
        }
    }

    fn visit_exprs(
        &self,
        exprs: &[Expr],
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        ctx: Context,
    ) {
        for expr in exprs {
            self.visit_expr(expr, line_index, findings, ctx);
        }
    }

    pub(super) fn check_target_safety(expr: &Expr) -> bool {
        if let Expr::Name(name) = expr {
            let lower = name.id.as_str().to_lowercase();
            if lower.contains("public")
                || lower.contains("example")
                || lower.contains("sample")
                || is_test_name(&lower)
                || lower.ends_with("_regex")
                || lower.ends_with("_pattern")
                || lower.ends_with("_re")
                || lower.ends_with("_fmt")
                || lower.ends_with("_format")
            {
                return true;
            }
            if lower.contains("jwt") && lower.contains("token") {
                return true;
            }
        }
        false
    }
}
