use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

use super::super::utils::{contains_sensitive_names, create_finding, get_call_name};

fn has_case_insensitive_suffix(value: &str, suffix: &str) -> bool {
    value
        .get(value.len().saturating_sub(suffix.len())..)
        .is_some_and(|tail| tail.eq_ignore_ascii_case(suffix))
}

/// Rule for detecting logging of potentially sensitive data.
pub struct LoggingSensitiveDataRule {
    /// Rule metadata.
    pub metadata: RuleMetadata,
}

impl LoggingSensitiveDataRule {
    /// Creates a new logging-sensitive-data rule instance.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}

impl Rule for LoggingSensitiveDataRule {
    fn name(&self) -> &'static str {
        "LoggingSensitiveDataRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }

    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name.starts_with("logging.")
                    || name.starts_with("logger.")
                    || name == "log"
                    || has_case_insensitive_suffix(&name, ".debug")
                    || has_case_insensitive_suffix(&name, ".info")
                    || has_case_insensitive_suffix(&name, ".warning")
                    || has_case_insensitive_suffix(&name, ".error")
                    || has_case_insensitive_suffix(&name, ".critical")
                {
                    for arg in &call.arguments.args {
                        if contains_sensitive_names(arg) {
                            return Some(vec![create_finding(
                                "Potential sensitive data in log statement. Avoid logging passwords, tokens, secrets, or API keys.",
                                self.metadata,
                                context,
                                call.range().start(),
                                "MEDIUM",
                            )]);
                        }
                    }
                }
            }
        }
        None
    }
}
