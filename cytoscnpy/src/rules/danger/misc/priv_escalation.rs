use super::super::utils::{create_finding, get_call_name};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

/// Rule for detecting privilege escalation via `os.setuid`, `os.setgid`, etc.
///
/// These calls permanently drop or elevate the process UID/GID and can be used
/// to gain root or escape sandboxes. PRIV726 / OWASP A04:2021.
pub struct PrivEscalationRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}

impl PrivEscalationRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}

impl Rule for PrivEscalationRule {
    fn name(&self) -> &'static str {
        "PrivEscalationRule"
    }

    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }

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

        // Check full qualified name (os.setuid) OR bare attribute (proc.setuid).
        // Both checks run independently — get_call_name returns "proc.setuid" (Some),
        // which would skip the attr branch in an if/else-if pattern.
        let name_is_priv = name_opt.as_deref().is_some_and(|n| {
            matches!(
                n,
                "os.setuid"
                    | "os.setgid"
                    | "os.setreuid"
                    | "os.setregid"
                    | "os.seteuid"
                    | "os.setegid"
                    | "os.setgroups"
            )
        });
        let attr_is_priv = attr_name.is_some_and(|a| {
            matches!(
                a,
                "setuid" | "setgid" | "setreuid" | "setregid" | "seteuid" | "setegid" | "setgroups"
            )
        });
        let is_priv_call = name_is_priv || attr_is_priv;

        if !is_priv_call {
            return None;
        }

        Some(vec![create_finding(
            "Potential privilege escalation: os.setuid/setgid call modifies process credentials. Ensure this is intentional and authorized.",
            self.metadata,
            context,
            call.range().start(),
            "HIGH",
        )])
    }
}
