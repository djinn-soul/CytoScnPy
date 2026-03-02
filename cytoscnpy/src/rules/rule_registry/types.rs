/// Canonical high-level category for a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuleCategory {
    /// Security/danger rule.
    Danger,
    /// Code quality/performance rule.
    Quality,
    /// Secrets-detection rule.
    Secrets,
}

impl RuleCategory {
    /// Returns the canonical display form for this category.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            RuleCategory::Danger => "Danger",
            RuleCategory::Quality => "Quality",
            RuleCategory::Secrets => "Secrets",
        }
    }
}

/// Default severity for a rule when no override applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuleSeverity {
    /// Highest severity.
    Critical,
    /// High severity.
    High,
    /// Medium severity.
    Medium,
    /// Low severity.
    Low,
}

impl RuleSeverity {
    /// Returns the canonical display form for this severity.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            RuleSeverity::Critical => "CRITICAL",
            RuleSeverity::High => "HIGH",
            RuleSeverity::Medium => "MEDIUM",
            RuleSeverity::Low => "LOW",
        }
    }
}

/// Strongly typed rule metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuleDescriptor {
    /// Stable rule identifier (for example `CSP-D101`).
    pub id: &'static str,
    /// Rule category.
    pub category: RuleCategory,
    /// Default severity for the rule.
    pub default_severity: RuleSeverity,
    /// Documentation URL/path for end-user guidance.
    pub docs_url: &'static str,
}

pub(super) const fn rule(
    id: &'static str,
    category: RuleCategory,
    default_severity: RuleSeverity,
    docs_url: &'static str,
) -> RuleDescriptor {
    RuleDescriptor {
        id,
        category,
        default_severity,
        docs_url,
    }
}
