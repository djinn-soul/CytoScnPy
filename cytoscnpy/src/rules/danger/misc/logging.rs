use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

use super::super::utils::{contains_sensitive_names, create_finding, get_call_name};

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

    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name.starts_with("logging.")
                    || name.starts_with("logger.")
                    || name == "log"
                    || name.ends_with(".debug")
                    || name.ends_with(".info")
                    || name.ends_with(".warning")
                    || name.ends_with(".error")
                    || name.ends_with(".critical")
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
