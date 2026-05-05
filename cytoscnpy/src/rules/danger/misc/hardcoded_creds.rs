use super::super::utils::create_finding;
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{CmpOp, Expr, ExprCompare};
use ruff_text_size::Ranged;

/// Known default/hardcoded credential values to flag.
const DEFAULT_CREDS: &[&str] = &[
    "admin",
    "administrator",
    "root",
    "password",
    "pass",
    "passwd",
    "1234",
    "12345",
    "123456",
    "password123",
    "qwerty",
    "letmein",
    "welcome",
    "changeme",
    "default",
    "guest",
    "test",
    "secret",
    "",
];

/// Variable name substrings that indicate credential/auth context.
const CRED_VAR_PATTERNS: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "pass",
    "user",
    "username",
    "login",
    "admin",
    "auth",
    "credential",
    "token",
    "secret",
];

fn contains_cred_pattern(name: &str) -> bool {
    let lower = name.to_lowercase();
    CRED_VAR_PATTERNS.iter().any(|p| lower.contains(p))
}

fn is_default_cred_literal(expr: &Expr) -> bool {
    if let Expr::StringLiteral(s) = expr {
        let val = s.value.to_str().to_lowercase();
        DEFAULT_CREDS.iter().any(|c| val == *c)
    } else {
        false
    }
}

/// Rule for detecting hardcoded default/admin credentials in comparison expressions.
///
/// Flags `if user == "admin"`, `if password == "password"`, etc.
/// AUTH711 / ADMIN795 / OWASP A07:2021.
pub struct HardcodedCredsRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}

impl HardcodedCredsRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}

impl Rule for HardcodedCredsRule {
    fn name(&self) -> &'static str {
        "HardcodedCredsRule"
    }

    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }

    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if context.is_test_file {
            return None;
        }
        let Expr::Compare(ExprCompare {
            left,
            ops,
            comparators,
            ..
        }) = expr
        else {
            return None;
        };

        // Only flag == and != comparisons
        if !ops.iter().any(|op| matches!(op, CmpOp::Eq | CmpOp::NotEq)) {
            return None;
        }

        let pairs: Vec<(&Expr, &Expr)> = std::iter::once(left.as_ref())
            .zip(comparators.iter())
            .collect();

        for (lhs, rhs) in pairs {
            // Pattern: cred_var == "default_value" or "default_value" == cred_var
            let (var_expr, lit_expr) = if matches!(rhs, Expr::StringLiteral(_)) {
                (lhs, rhs)
            } else if matches!(lhs, Expr::StringLiteral(_)) {
                (rhs, lhs)
            } else {
                continue;
            };

            if !is_default_cred_literal(lit_expr) {
                continue;
            }

            // Check if the variable name suggests credentials
            let var_name = match var_expr {
                Expr::Name(n) => n.id.as_str(),
                Expr::Attribute(a) => a.attr.as_str(),
                _ => continue,
            };

            if contains_cred_pattern(var_name) {
                return Some(vec![create_finding(
                    "Hardcoded default credential in comparison. Never compare credentials against well-known defaults; use secure credential storage.",
                    self.metadata,
                    context,
                    expr.range().start(),
                    "HIGH",
                )]);
            }
        }

        None
    }
}
