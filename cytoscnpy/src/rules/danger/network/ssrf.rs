use super::super::utils::{create_finding, get_call_name, is_arg_literal, is_literal_expr};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

/// Rule for detecting dynamic URLs flowing into network request APIs.
pub struct SSRFRule {
    /// Rule metadata.
    pub metadata: RuleMetadata,
}
impl SSRFRule {
    /// Creates a new SSRF rule instance.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for SSRFRule {
    fn name(&self) -> &'static str {
        "SSRFRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name.starts_with("requests.")
                    || name.starts_with("httpx.")
                    || name == "urllib.request.urlopen"
                {
                    let mut findings = Vec::new();
                    if !call.arguments.args.is_empty() {
                        if name.ends_with(".request") {
                            if call.arguments.args.len() >= 2
                                && !is_literal_expr(&call.arguments.args[1])
                            {
                                findings.push(create_finding(
                                    "Potential SSRF (dynamic URL in positional arg 2)",
                                    self.metadata,
                                    context,
                                    call.range().start(),
                                    "CRITICAL",
                                ));
                            }
                        } else if !is_arg_literal(&call.arguments.args, 0) {
                            findings.push(create_finding(
                                "Potential SSRF (dynamic URL in positional arg)",
                                self.metadata,
                                context,
                                call.range().start(),
                                "CRITICAL",
                            ));
                        }
                    }
                    for keyword in &call.arguments.keywords {
                        if let Some(arg) = &keyword.arg {
                            let arg_name = arg.as_str();
                            if matches!(arg_name, "url" | "uri" | "address")
                                && !is_literal_expr(&keyword.value)
                            {
                                findings.push(create_finding(
                                    &format!("Potential SSRF (dynamic URL in '{arg_name}' arg)"),
                                    self.metadata,
                                    context,
                                    call.range().start(),
                                    "CRITICAL",
                                ));
                            }
                        }
                    }
                    if !findings.is_empty() {
                        return Some(findings);
                    }
                }
            }
        }
        None
    }
}
