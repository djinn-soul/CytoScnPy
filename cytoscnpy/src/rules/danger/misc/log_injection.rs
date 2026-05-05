use super::super::utils::{create_finding, get_call_name};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{Expr, ExprBinOp, Operator};
use ruff_text_size::Ranged;

/// Returns true if the expression is a potential log-injection vector:
/// an f-string with dynamic parts, string concatenation, or `%`-formatting.
fn is_injectable_log_arg(expr: &Expr) -> bool {
    match expr {
        // f-string with at least one dynamic component
        Expr::FString(f) => f.value.iter().any(|part| {
            use ruff_python_ast::FStringPart;
            matches!(part, FStringPart::FString(_))
        }),
        // string + anything or anything + string
        Expr::BinOp(ExprBinOp {
            op: Operator::Add,
            left,
            right,
            ..
        }) => {
            matches!(**left, Expr::StringLiteral(_) | Expr::FString(_))
                || matches!(**right, Expr::StringLiteral(_) | Expr::FString(_))
                || is_injectable_log_arg(left)
                || is_injectable_log_arg(right)
        }
        // "prefix %s" % value
        Expr::BinOp(ExprBinOp {
            op: Operator::Mod,
            left,
            ..
        }) => matches!(**left, Expr::StringLiteral(_)),
        _ => false,
    }
}

/// Rule for detecting log injection via unsanitized user input.
///
/// Flags log calls where the argument is an f-string, string concatenation,
/// or `%`-format string — all vectors for newline injection (LOG741).
/// Attackers embed `\n` to forge fake log entries. CSP-D904 / OWASP A09:2021.
pub struct LogInjectionRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}

impl LogInjectionRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}

impl Rule for LogInjectionRule {
    fn name(&self) -> &'static str {
        "LogInjectionRule"
    }

    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }

    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        let Expr::Call(call) = expr else {
            return None;
        };

        let name_opt = get_call_name(&call.func);

        fn has_case_insensitive_suffix(value: &str, suffix: &str) -> bool {
            value
                .get(value.len().saturating_sub(suffix.len())..)
                .is_some_and(|tail| tail.eq_ignore_ascii_case(suffix))
        }

        let is_log_call = name_opt.as_deref().is_some_and(|name| {
            name.starts_with("logging.")
                || name.starts_with("logger.")
                || name == "log"
                || has_case_insensitive_suffix(name, ".debug")
                || has_case_insensitive_suffix(name, ".info")
                || has_case_insensitive_suffix(name, ".warning")
                || has_case_insensitive_suffix(name, ".error")
                || has_case_insensitive_suffix(name, ".critical")
        });

        if !is_log_call {
            return None;
        }

        for arg in &call.arguments.args {
            if is_injectable_log_arg(arg) {
                return Some(vec![create_finding(
                    "Potential log injection: unsanitized dynamic content in log statement. Strip or escape newlines (\\n, \\r) before logging user-controlled data.",
                    self.metadata,
                    context,
                    call.range().start(),
                    "MEDIUM",
                )]);
            }
        }

        None
    }
}
