use super::{finding::create_finding, CAT_BEST_PRACTICES};
use crate::rules::ids;
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextSize};
const META_MUTABLE_DEFAULT: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_MUTABLE_DEFAULT,
    category: CAT_BEST_PRACTICES,
};
const META_BARE_EXCEPT: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_BARE_EXCEPT,
    category: CAT_BEST_PRACTICES,
};
const META_DANGEROUS_COMPARISON: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_DANGEROUS_COMPARISON,
    category: CAT_BEST_PRACTICES,
};
pub(super) struct MutableDefaultArgumentRule;
impl Rule for MutableDefaultArgumentRule {
    fn name(&self) -> &'static str {
        "MutableDefaultArgumentRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_MUTABLE_DEFAULT
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        let parameters = match stmt {
            Stmt::FunctionDef(f) => &f.parameters,
            _ => return None,
        };
        let mut findings = Vec::new();
        let check_arg = |arg: &ast::ParameterWithDefault, findings: &mut Vec<Finding>| {
            if let Some(default) = &arg.default {
                if is_mutable(default) {
                    findings.push(create_finding(
                        "Mutable default argument (use None and check inside function)",
                        META_MUTABLE_DEFAULT,
                        context,
                        default.range().start(),
                        "MEDIUM",
                    ));
                }
            }
        };
        for arg in &parameters.posonlyargs {
            check_arg(arg, &mut findings);
        }
        for arg in &parameters.args {
            check_arg(arg, &mut findings);
        }
        for arg in &parameters.kwonlyargs {
            check_arg(arg, &mut findings);
        }
        if findings.is_empty() {
            None
        } else {
            Some(findings)
        }
    }
}
fn is_mutable(expr: &Expr) -> bool {
    match expr {
        Expr::List(_) | Expr::Dict(_) | Expr::Set(_) => true,
        Expr::Call(call) => {
            if let Expr::Name(name) = &*call.func {
                matches!(name.id.as_str(), "list" | "dict" | "set")
            } else {
                false
            }
        }
        _ => false,
    }
}
pub(super) struct BareExceptRule;
impl Rule for BareExceptRule {
    fn name(&self) -> &'static str {
        "BareExceptRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_BARE_EXCEPT
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        if let Stmt::Try(t) = stmt {
            for handler in &t.handlers {
                let ast::ExceptHandler::ExceptHandler(h) = handler;
                if h.type_.is_none() {
                    return Some(vec![create_finding(
                        "Bare except block (catch specific exceptions)",
                        META_BARE_EXCEPT,
                        context,
                        h.range().start(),
                        "LOW",
                    )]);
                }
            }
        }
        None
    }
}
pub(super) struct DangerousComparisonRule;
impl Rule for DangerousComparisonRule {
    fn name(&self) -> &'static str {
        "DangerousComparisonRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_DANGEROUS_COMPARISON
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        let Expr::Compare(comp) = expr else {
            return None;
        };
        let mut findings = Vec::new();
        let mut left: &Expr = comp.left.as_ref();
        for (op, right) in comp.ops.iter().zip(comp.comparators.iter()) {
            if !matches!(op, ast::CmpOp::Eq | ast::CmpOp::NotEq) {
                left = right;
                continue;
            }
            if let Some(location) = dangerous_literal_location(left, right) {
                findings.push(create_finding(
                    "Dangerous comparison to True/False/None (use 'is' or 'is not')",
                    META_DANGEROUS_COMPARISON,
                    context,
                    location,
                    "LOW",
                ));
            }
            left = right;
        }
        if findings.is_empty() {
            None
        } else {
            Some(findings)
        }
    }
}
fn dangerous_literal_location(left: &Expr, right: &Expr) -> Option<TextSize> {
    if is_bool_or_none(left) {
        return Some(left.range().start());
    }
    if is_bool_or_none(right) {
        return Some(right.range().start());
    }
    None
}
fn is_bool_or_none(expr: &Expr) -> bool {
    matches!(expr, Expr::BooleanLiteral(_) | Expr::NoneLiteral(_))
}

