use std::collections::HashMap;
use std::sync::OnceLock;

use super::catalog::danger::DANGER_RULES;
use super::catalog::quality::QUALITY_RULES;
use super::catalog::secrets::SECRETS_RULES;
use super::types::RuleDescriptor;

static ALL_RULES: OnceLock<Vec<RuleDescriptor>> = OnceLock::new();
static RULE_REGISTRY_BY_ID: OnceLock<HashMap<&'static str, &'static RuleDescriptor>> =
    OnceLock::new();

fn all_rules_vec() -> &'static Vec<RuleDescriptor> {
    ALL_RULES.get_or_init(|| {
        let mut rules =
            Vec::with_capacity(DANGER_RULES.len() + QUALITY_RULES.len() + SECRETS_RULES.len());
        rules.extend_from_slice(DANGER_RULES);
        rules.extend_from_slice(QUALITY_RULES);
        rules.extend_from_slice(SECRETS_RULES);
        rules
    })
}

/// Returns all known rule descriptors.
#[must_use]
pub fn all_rule_descriptors() -> &'static [RuleDescriptor] {
    all_rules_vec().as_slice()
}

/// Returns the ID-indexed rule descriptor map.
#[must_use]
pub fn rule_registry_by_id() -> &'static HashMap<&'static str, &'static RuleDescriptor> {
    RULE_REGISTRY_BY_ID.get_or_init(|| {
        all_rule_descriptors()
            .iter()
            .map(|rule| (rule.id, rule))
            .collect()
    })
}

/// Looks up a rule descriptor by rule ID.
#[must_use]
pub fn get_rule_descriptor(rule_id: &str) -> Option<&'static RuleDescriptor> {
    rule_registry_by_id().get(rule_id).copied()
}
