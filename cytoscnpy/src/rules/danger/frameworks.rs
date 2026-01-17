use super::utils::create_finding;
use crate::rules::{Context, Finding, Rule};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

pub struct DjangoSecurityRule;
impl Rule for DjangoSecurityRule {
    fn name(&self) -> &'static str {
        "DjangoSecurityRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D904"
    }
    /// Detects hardcoded SECRET_KEY in assignments
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        if let Stmt::Assign(assign) = stmt {
            for target in &assign.targets {
                if let Expr::Name(n) = target {
                    if n.id.as_str() == "SECRET_KEY" {
                        if let Expr::StringLiteral(_) = &*assign.value {
                            return Some(vec![create_finding(
                                "Hardcoded SECRET_KEY detected. Store secrets in environment variables.",
                                self.code(),
                                context,
                                assign.value.range().start(),
                                "CRITICAL",
                            )]);
                        }
                    }
                }
            }
        }
        None
    }
}
