use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

use super::super::crypto::check_ciphers_and_modes;
use super::super::network::check_network_and_ssl;
use super::super::utils::{create_finding, get_call_name};
use super::metadata::META_INPUT;

/// Rule for detecting blacklisted function calls.
pub struct BlacklistCallRule {
    /// Rule metadata.
    pub metadata: RuleMetadata,
}

impl BlacklistCallRule {
    /// Creates a new blacklist-call rule instance.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}

impl Rule for BlacklistCallRule {
    fn name(&self) -> &'static str {
        "BlacklistCallRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }

    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if let Some(finding) = check_ciphers_and_modes(&name, call, context) {
                    return Some(vec![finding]);
                }
                if let Some(finding) = check_network_and_ssl(&name, call, context) {
                    return Some(vec![finding]);
                }
                if let Some(finding) = check_misc_blacklist(&name, call, context) {
                    return Some(vec![finding]);
                }
            }
        }
        None
    }
}

fn check_misc_blacklist(name: &str, call: &ast::ExprCall, context: &Context) -> Option<Finding> {
    use super::super::filesystem::META_TEMPNAM;
    use super::super::injection::META_MARK_SAFE;

    if name == "mark_safe" || name == "django.utils.safestring.mark_safe" {
        return Some(create_finding(
            "Use of mark_safe() may expose XSS. Review carefully.",
            META_MARK_SAFE,
            context,
            call.range().start(),
            "MEDIUM",
        ));
    }
    if name == "input" {
        return Some(create_finding(
            "Check for use of input() (vulnerable in Py2, unsafe in Py3 if not careful).",
            META_INPUT,
            context,
            call.range().start(),
            "HIGH",
        ));
    }
    if name == "os.tempnam" || name == "os.tmpnam" {
        return Some(create_finding(
            "Use of os.tempnam/os.tmpnam is vulnerable to symlink attacks. Use tempfile module instead.",
            META_TEMPNAM,
            context,
            call.range().start(),
            "MEDIUM",
        ));
    }
    None
}
