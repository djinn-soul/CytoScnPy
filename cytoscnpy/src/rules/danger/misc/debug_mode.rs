use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

use super::super::utils::{create_finding, get_call_name};

/// Rule for detecting if debug mode is enabled in production.
pub struct DebugModeRule {
    /// Rule metadata.
    pub metadata: RuleMetadata,
}

impl DebugModeRule {
    /// Creates a new debug-mode rule instance.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}

impl Rule for DebugModeRule {
    fn name(&self) -> &'static str {
        "DebugModeRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }

    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name.ends_with(".run") || name == "run_simple" {
                    for keyword in &call.arguments.keywords {
                        if let Some(arg) = &keyword.arg {
                            if arg == "debug" {
                                if let Expr::BooleanLiteral(value) = &keyword.value {
                                    if value.value {
                                        return Some(vec![create_finding(
                                            "Debug mode enabled (debug=True) in production",
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
