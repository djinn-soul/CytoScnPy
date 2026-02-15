//! Typed metadata registry for all rule IDs.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::rules::ids;

const DOC_DANGEROUS_CODE: &str = "docs/dangerous-code.md";
const DOC_QUALITY: &str = "docs/quality.md";
const DOC_SECURITY: &str = "docs/security.md";

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

const fn rule(
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

/// Static registry of known rule descriptors.
pub static RULE_REGISTRY: &[RuleDescriptor] = &[
    rule(
        ids::RULE_ID_EVAL,
        RuleCategory::Danger,
        RuleSeverity::Critical,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_EXEC,
        RuleCategory::Danger,
        RuleSeverity::Critical,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_SUBPROCESS,
        RuleCategory::Danger,
        RuleSeverity::Critical,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_ASYNC_SUBPROCESS,
        RuleCategory::Danger,
        RuleSeverity::Critical,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_INPUT,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_SQL_INJECTION,
        RuleCategory::Danger,
        RuleSeverity::Critical,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_SQL_RAW,
        RuleCategory::Danger,
        RuleSeverity::Critical,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_XSS,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_XML,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_MARK_SAFE,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_PICKLE,
        RuleCategory::Danger,
        RuleSeverity::Critical,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_YAML,
        RuleCategory::Danger,
        RuleSeverity::Critical,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_MARSHAL,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_MODEL_DESER,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_MD5,
        RuleCategory::Danger,
        RuleSeverity::Medium,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_SHA1,
        RuleCategory::Danger,
        RuleSeverity::Medium,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_CIPHER,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_MODE,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_RANDOM,
        RuleCategory::Danger,
        RuleSeverity::Medium,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_REQUESTS,
        RuleCategory::Danger,
        RuleSeverity::Medium,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_SSRF,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_DEBUG_MODE,
        RuleCategory::Danger,
        RuleSeverity::Medium,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_BIND_ALL,
        RuleCategory::Danger,
        RuleSeverity::Medium,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_TIMEOUT,
        RuleCategory::Danger,
        RuleSeverity::Low,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_FTP,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_HTTPS_CONNECTION,
        RuleCategory::Danger,
        RuleSeverity::Medium,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_SSL_UNVERIFIED,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_TELNET,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_URL_OPEN,
        RuleCategory::Danger,
        RuleSeverity::Medium,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_WRAP_SOCKET,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_PATH_TRAVERSAL,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_TARFILE,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_ZIPFILE,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_TEMPFILE,
        RuleCategory::Danger,
        RuleSeverity::Medium,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_PERMISSIONS,
        RuleCategory::Danger,
        RuleSeverity::Medium,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_TEMPNAM,
        RuleCategory::Danger,
        RuleSeverity::Medium,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_METHOD_MISUSE,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_ASSERT,
        RuleCategory::Danger,
        RuleSeverity::Low,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_INSECURE_IMPORT,
        RuleCategory::Danger,
        RuleSeverity::Medium,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_JINJA_AUTOESCAPE,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_BLACKLIST,
        RuleCategory::Danger,
        RuleSeverity::Medium,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_OPEN_REDIRECT,
        RuleCategory::Danger,
        RuleSeverity::Medium,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_LOGGING_SENSITIVE,
        RuleCategory::Danger,
        RuleSeverity::Medium,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_DJANGO_SECURITY,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_XSS_GENERIC,
        RuleCategory::Danger,
        RuleSeverity::High,
        DOC_DANGEROUS_CODE,
    ),
    rule(
        ids::RULE_ID_MUTABLE_DEFAULT,
        RuleCategory::Quality,
        RuleSeverity::Medium,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_BARE_EXCEPT,
        RuleCategory::Quality,
        RuleSeverity::Medium,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_DANGEROUS_COMPARISON,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_COMPLEXITY,
        RuleCategory::Quality,
        RuleSeverity::Medium,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_NESTING,
        RuleCategory::Quality,
        RuleSeverity::Medium,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_MIN_MI,
        RuleCategory::Quality,
        RuleSeverity::Medium,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_COGNITIVE_COMPLEXITY,
        RuleCategory::Quality,
        RuleSeverity::Medium,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_COHESION,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_ARGUMENT_COUNT,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_FUNCTION_LENGTH,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_MEMBERSHIP_LIST,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_FILE_READ_RISK,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_STRING_CONCAT,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_USELESS_CAST,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_REGEX_LOOP,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_ATTRIBUTE_HOIST,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_PURE_CALL_HOIST,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_EXCEPTION_FLOW_LOOP,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_DICT_ITERATOR,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_GLOBAL_LOOP,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_MEMORYVIEW_BYTES,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_TUPLE_OVER_LIST,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_COMPREHENSION,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_PANDAS_CHUNK_RISK,
        RuleCategory::Quality,
        RuleSeverity::Low,
        DOC_QUALITY,
    ),
    rule(
        ids::RULE_ID_SECRET_PATTERN,
        RuleCategory::Secrets,
        RuleSeverity::High,
        DOC_SECURITY,
    ),
    rule(
        ids::RULE_ID_SECRET_ASSIGNMENT,
        RuleCategory::Secrets,
        RuleSeverity::Medium,
        DOC_SECURITY,
    ),
];

static RULE_REGISTRY_BY_ID: OnceLock<HashMap<&'static str, &'static RuleDescriptor>> =
    OnceLock::new();

/// Returns all known rule descriptors.
#[must_use]
pub fn all_rule_descriptors() -> &'static [RuleDescriptor] {
    RULE_REGISTRY
}

/// Returns the ID-indexed rule descriptor map.
#[must_use]
pub fn rule_registry_by_id() -> &'static HashMap<&'static str, &'static RuleDescriptor> {
    RULE_REGISTRY_BY_ID.get_or_init(|| RULE_REGISTRY.iter().map(|rule| (rule.id, rule)).collect())
}

/// Looks up a rule descriptor by rule ID.
#[must_use]
pub fn get_rule_descriptor(rule_id: &str) -> Option<&'static RuleDescriptor> {
    rule_registry_by_id().get(rule_id).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_contains_known_rule_with_metadata() {
        let descriptor = get_rule_descriptor(ids::RULE_ID_SQL_INJECTION).unwrap();
        assert_eq!(descriptor.category, RuleCategory::Danger);
        assert_eq!(descriptor.default_severity, RuleSeverity::Critical);
        assert_eq!(descriptor.docs_url, DOC_DANGEROUS_CODE);
    }

    #[test]
    fn test_registry_contains_secrets_rules() {
        let descriptor = get_rule_descriptor(ids::RULE_ID_SECRET_PATTERN).unwrap();
        assert_eq!(descriptor.category.as_str(), "Secrets");
        assert_eq!(descriptor.default_severity.as_str(), "HIGH");
        assert_eq!(descriptor.docs_url, DOC_SECURITY);
    }
}
