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

/// Rule for detecting `@csrf_exempt` decorator — disables CSRF protection.
pub const META_CSRF_EXEMPT: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_CSRF_EXEMPT,
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

/// Rule for detecting `@csrf_exempt` decorator on Django views.
///
/// `csrf_exempt` disables CSRF protection for a view entirely. Any POST/PUT/DELETE
/// endpoint with this decorator is vulnerable to cross-site request forgery.
/// G404 / OWASP A05:2021.
pub struct CsrfExemptRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}

impl CsrfExemptRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}

impl Rule for CsrfExemptRule {
    fn name(&self) -> &'static str {
        "CsrfExemptRule"
    }

    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }

    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        let decorators = match stmt {
            Stmt::FunctionDef(f) => &f.decorator_list,
            Stmt::ClassDef(c) => &c.decorator_list,
            _ => return None,
        };

        for decorator in decorators {
            if is_csrf_exempt_expr(&decorator.expression) {
                return Some(vec![create_finding(
                    "@csrf_exempt disables CSRF protection for this view. All state-changing requests are vulnerable to CSRF attacks.",
                    self.metadata,
                    context,
                    decorator.range().start(),
                    "HIGH",
                )]);
            }
        }
        None
    }
}

/// Returns true if the decorator expression resolves to `csrf_exempt`.
fn is_csrf_exempt_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Name(n) => n.id.as_str() == "csrf_exempt",
        Expr::Attribute(a) => a.attr.as_str() == "csrf_exempt",
        Expr::Call(call) => is_csrf_exempt_expr(&call.func),
        _ => false,
    }
}
