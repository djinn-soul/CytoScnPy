use super::utils::create_finding;
use crate::rules::ids;
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{Expr, Stmt};
use ruff_text_size::Ranged;

/// Rule for detecting insecure Django configurations.
pub const META_DJANGO_SECURITY: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_DJANGO_SECURITY,
    category: super::CAT_PRIVACY,
};

/// django security rule
pub struct DjangoSecurityRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl DjangoSecurityRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for DjangoSecurityRule {
    fn name(&self) -> &'static str {
        "DjangoSecurityRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    /// Detects hardcoded `SECRET_KEY` in assignments
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        if let Stmt::Assign(assign) = stmt {
            for target in &assign.targets {
                if let Expr::Name(n) = target {
                    if n.id.as_str() == "SECRET_KEY" {
                        if let Expr::StringLiteral(_) = &*assign.value {
                            return Some(vec![create_finding(
                                "Hardcoded SECRET_KEY detected. Store secrets in environment variables.",
                                self.metadata,
                                context,
                                assign.value.range().start(),
                                "CRITICAL",
                            )]);
                        }
                    }
                }
            }
        }
        None
    }
}
