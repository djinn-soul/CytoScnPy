//! `XPath` injection detection for lxml and `ElementPath`.

use super::super::utils::{create_finding, get_call_name, is_literal_expr};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

/// Rule for detecting `XPath` injection via non-literal `XPath` expressions.
///
/// Detects calls to `lxml.etree.XPath(expr)`, `tree.xpath(expr)`, and
/// `ElementPath.xpath(expr)` where the expression argument is non-literal.
/// XPATH720 / OWASP A03:2021.
pub struct XPathInjectionRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}

impl XPathInjectionRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}

impl Rule for XPathInjectionRule {
    fn name(&self) -> &'static str {
        "XPathInjectionRule"
    }

    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }

    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        let Expr::Call(call) = expr else {
            return None;
        };

        let name_opt = get_call_name(&call.func);
        let attr_name = if let Expr::Attribute(attr) = &*call.func {
            Some(attr.attr.as_str())
        } else {
            None
        };

        let is_xpath_call = match (&name_opt, attr_name) {
            (Some(name), _) => {
                name == "XPath"
                    || name == "etree.XPath"
                    || name == "lxml.etree.XPath"
                    || name.ends_with(".XPath")
                    || name.ends_with(".xpath")
            }
            (None, Some(attr)) => attr == "XPath" || attr == "xpath",
            _ => false,
        };

        if !is_xpath_call {
            return None;
        }

        if let Some(arg) = call.arguments.args.first() {
            if !is_literal_expr(arg) {
                return Some(vec![create_finding(
                    "Potential XPath injection: non-literal XPath expression. Sanitize user input before constructing XPath queries.",
                    self.metadata,
                    context,
                    call.range().start(),
                    "HIGH",
                )]);
            }
        }

        for kw in &call.arguments.keywords {
            if kw
                .arg
                .as_ref()
                .is_some_and(|a| matches!(a.as_str(), "path" | "xpath"))
                && !is_literal_expr(&kw.value)
            {
                return Some(vec![create_finding(
                    "Potential XPath injection: non-literal XPath expression. Sanitize user input before constructing XPath queries.",
                    self.metadata,
                    context,
                    call.range().start(),
                    "HIGH",
                )]);
            }
        }

        None
    }
}
