use super::super::utils::{create_finding, get_call_name};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

/// Rule for detecting request calls without safe timeout settings.
pub struct RequestWithoutTimeoutRule {
    /// Rule metadata.
    pub metadata: RuleMetadata,
}
impl RequestWithoutTimeoutRule {
    /// Creates a new timeout rule instance.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for RequestWithoutTimeoutRule {
    fn name(&self) -> &'static str {
        "RequestWithoutTimeoutRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if (name.starts_with("requests.") || name.starts_with("httpx."))
                    && (name.ends_with(".get")
                        || name.ends_with(".post")
                        || name.ends_with(".put")
                        || name.ends_with(".delete")
                        || name.ends_with(".head")
                        || name.ends_with(".patch")
                        || name.ends_with(".request"))
                {
                    let mut bad_timeout = true;
                    for keyword in &call.arguments.keywords {
                        if keyword.arg.as_ref().is_some_and(|arg| arg == "timeout") {
                            bad_timeout = match &keyword.value {
                                Expr::NoneLiteral(_) => true,
                                Expr::BooleanLiteral(boolean) => !boolean.value,
                                Expr::NumberLiteral(number) => match &number.value {
                                    ast::Number::Int(int) => int.to_string() == "0",
                                    ast::Number::Float(float) => *float == 0.0,
                                    ast::Number::Complex { .. } => false,
                                },
                                _ => false,
                            };
                            if !bad_timeout {
                                break;
                            }
                        }
                    }
                    if bad_timeout {
                        return Some(vec![create_finding(
                            "Request call without timeout or with an unsafe timeout (None, 0, False). This can cause the process to hang indefinitely.",
                            self.metadata,
                            context,
                            call.range().start(),
                            "MEDIUM",
                        )]);
                    }
                }
            }
        }
        None
    }
}
