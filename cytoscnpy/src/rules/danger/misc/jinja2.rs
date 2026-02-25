use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

use super::super::utils::{create_finding, get_call_name};

/// Rule for detecting disabled autoescaping in Jinja2 templates.
pub struct Jinja2AutoescapeRule {
    /// Rule metadata.
    pub metadata: RuleMetadata,
}

impl Jinja2AutoescapeRule {
    /// Creates a new Jinja2 autoescape rule instance.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}

impl Rule for Jinja2AutoescapeRule {
    fn name(&self) -> &'static str {
        "Jinja2AutoescapeRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }

    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "jinja2.Environment" || name == "Environment" {
                    for keyword in &call.arguments.keywords {
                        if let Some(arg) = &keyword.arg {
                            if arg == "autoescape" {
                                if let Expr::BooleanLiteral(value) = &keyword.value {
                                    if !value.value {
                                        return Some(vec![create_finding(
                                            "jinja2.Environment created with autoescape=False. This enables XSS attacks.",
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
