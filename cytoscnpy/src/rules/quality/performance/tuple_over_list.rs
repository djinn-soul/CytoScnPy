use super::{create_finding, META_TUPLE_OVER_LIST};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

pub(in crate::rules::quality) struct UseTupleOverListRule {
    function_depth: usize,
}

impl UseTupleOverListRule {
    pub fn new() -> Self {
        Self { function_depth: 0 }
    }
}

impl Rule for UseTupleOverListRule {
    fn name(&self) -> &'static str {
        "UseTupleOverListRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_TUPLE_OVER_LIST
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        if matches!(stmt, Stmt::FunctionDef(_)) {
            self.function_depth += 1;
            return None;
        }

        if self.function_depth > 0 {
            return None;
        }

        match stmt {
            Stmt::Assign(assign) => {
                Self::check_assign_targets(&assign.targets, &assign.value, context)
            }
            Stmt::AnnAssign(assign) => assign
                .value
                .as_ref()
                .and_then(|value| Self::check_target(&assign.target, value, context)),
            _ => None,
        }
    }

    fn leave_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        if matches!(stmt, Stmt::FunctionDef(_)) {
            self.function_depth = self.function_depth.saturating_sub(1);
        }
        None
    }
}

impl UseTupleOverListRule {
    fn check_assign_targets(
        targets: &[Expr],
        value: &Expr,
        context: &Context,
    ) -> Option<Vec<Finding>> {
        let Expr::List(list) = value else {
            return None;
        };

        if list.elts.is_empty() || !list.elts.iter().all(is_immutable_literal) {
            return None;
        }

        let mut findings = Vec::new();
        for target in targets {
            if let Some(finding) = Self::check_target(target, value, context) {
                findings.extend(finding);
            }
        }

        if findings.is_empty() {
            None
        } else {
            Some(findings)
        }
    }

    fn check_target(target: &Expr, value: &Expr, context: &Context) -> Option<Vec<Finding>> {
        let Expr::List(list) = value else {
            return None;
        };

        let Expr::Name(name) = target else {
            return None;
        };

        let constant_name = name.id.as_str();
        if !is_constant_name(constant_name) {
            return None;
        }

        if list.elts.is_empty() || !list.elts.iter().all(is_immutable_literal) {
            return None;
        }

        Some(vec![create_finding(
            &format!(
                "Constant list '{constant_name}' looks immutable (use a tuple for lower overhead)"
            ),
            META_TUPLE_OVER_LIST,
            context,
            list.range().start(),
            "LOW",
        )])
    }
}

fn is_constant_name(name: &str) -> bool {
    name.len() > 1 && name.chars().all(|c| c.is_uppercase() || c == '_')
}

fn is_immutable_literal(expr: &Expr) -> bool {
    match expr {
        Expr::StringLiteral(_)
        | Expr::BytesLiteral(_)
        | Expr::NumberLiteral(_)
        | Expr::BooleanLiteral(_)
        | Expr::NoneLiteral(_)
        | Expr::EllipsisLiteral(_) => true,
        Expr::Tuple(tuple) => tuple.elts.iter().all(is_immutable_literal),
        Expr::UnaryOp(unary) => {
            matches!(unary.op, ast::UnaryOp::UAdd | ast::UnaryOp::USub)
                && matches!(&*unary.operand, Expr::NumberLiteral(_))
        }
        _ => false,
    }
}
