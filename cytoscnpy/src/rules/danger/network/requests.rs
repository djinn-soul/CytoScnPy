use super::super::utils::{create_finding, get_call_name};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

/// Rule for detecting `requests.*` calls with disabled SSL verification.
pub struct RequestsRule {
    /// Rule metadata.
    pub metadata: RuleMetadata,
}
impl RequestsRule {
    /// Creates a new requests rule instance.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for RequestsRule {
    fn name(&self) -> &'static str {
        "RequestsRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name.starts_with("requests.") {
                    for keyword in &call.arguments.keywords {
                        if let Some(arg) = &keyword.arg {
                            if arg == "verify" {
                                if let Expr::BooleanLiteral(boolean) = &keyword.value {
                                    if !boolean.value {
                                        return Some(vec![create_finding(
                                            "SSL verification disabled (verify=False)",
                                            self.metadata,
                                            context,
                                            call.range().start(),
                                            "HIGH",
                                        )]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
}
