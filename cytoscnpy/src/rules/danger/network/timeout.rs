use super::super::utils::{create_finding, get_call_name};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

fn has_case_sensitive_suffix(value: &str, suffix: &str) -> bool {
    value
        .get(value.len().saturating_sub(suffix.len())..)
        .is_some_and(|tail| tail == suffix)
}

fn is_request_method_call(name: &str) -> bool {
    has_case_sensitive_suffix(name, ".get")
        || has_case_sensitive_suffix(name, ".post")
        || has_case_sensitive_suffix(name, ".put")
        || has_case_sensitive_suffix(name, ".delete")
        || has_case_sensitive_suffix(name, ".head")
        || has_case_sensitive_suffix(name, ".patch")
        || has_case_sensitive_suffix(name, ".request")
}

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
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if (name.starts_with("requests.") || name.starts_with("httpx."))
                    && is_request_method_call(&name)
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
