//! Insecure temporary file and file permission detection.

use super::super::utils::{create_finding, get_call_name};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

/// Rule for detecting insecure temporary file usage.
pub struct TempfileRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl TempfileRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for TempfileRule {
    fn name(&self) -> &'static str {
        "TempfileRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                // Note: tempnam/tmpnam are handled by BlacklistCallRule (CSP-D506) to avoid overlap
                if name == "tempfile.mktemp" || name == "mktemp" || name.ends_with(".mktemp") {
                    return Some(vec![create_finding(
                        "Insecure use of tempfile.mktemp (race condition risk). Use tempfile.mkstemp or tempfile.TemporaryFile.",
                        self.metadata,
                        context,
                        call.range().start(),
                        "HIGH",
                    )]);
                }
            }
        }
        None
    }
}

/// Rule for detecting insecure file permission settings.
pub struct BadFilePermissionsRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl BadFilePermissionsRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for BadFilePermissionsRule {
    fn name(&self) -> &'static str {
        "BadFilePermissionsRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "os.chmod" {
                    let mode_arg = if call.arguments.args.len() >= 2 {
                        Some(&call.arguments.args[1])
                    } else {
                        call.arguments
                            .keywords
                            .iter()
                            .find(|k| k.arg.as_ref().is_some_and(|a| a == "mode"))
                            .map(|k| &k.value)
                    };

                    if let Some(mode) = mode_arg {
                        if let Expr::Attribute(attr) = mode {
                            if attr.attr.as_str() == "S_IWOTH" {
                                return Some(vec![create_finding(
                                    "Setting file permissions to world-writable (S_IWOTH) is insecure.",
                                    self.metadata,
                                    context,
                                    call.range().start(),
                                    "HIGH",
                                )]);
                            }
                        } else if let Expr::NumberLiteral(n) = mode {
                            if let ast::Number::Int(i) = &n.value {
                                if i.to_string() == "511" {
                                    return Some(vec![create_finding(
                                        "Setting file permissions to world-writable (0o777) is insecure.",
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
        None
    }
}
