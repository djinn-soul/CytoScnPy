//! Typed metadata registry for all rule IDs.

mod catalog;
mod lookup;
mod types;

pub use catalog::{DOC_DANGEROUS_CODE, DOC_QUALITY, DOC_SECURITY};
pub use lookup::{all_rule_descriptors, get_rule_descriptor, rule_registry_by_id};
pub use types::{RuleCategory, RuleDescriptor, RuleSeverity};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ids;

    #[test]
    fn test_registry_contains_known_rule_with_metadata() {
        let descriptor = get_rule_descriptor(ids::RULE_ID_SQL_INJECTION)
            .expect("expected SQL injection rule to be present");
        assert_eq!(descriptor.category, RuleCategory::Danger);
        assert_eq!(descriptor.default_severity, RuleSeverity::Critical);
        assert_eq!(descriptor.docs_url, DOC_DANGEROUS_CODE);
    }

    #[test]
    fn test_registry_contains_secrets_rules() {
        let descriptor = get_rule_descriptor(ids::RULE_ID_SECRET_PATTERN)
            .expect("expected secret pattern rule to be present");
        assert_eq!(descriptor.category.as_str(), "Secrets");
        assert_eq!(descriptor.default_severity.as_str(), "HIGH");
        assert_eq!(descriptor.docs_url, DOC_SECURITY);
    }
}
