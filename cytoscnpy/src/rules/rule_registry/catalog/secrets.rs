use crate::rules::ids;

use super::{rule, RuleCategory, RuleDescriptor, RuleSeverity, DOC_SECURITY};

pub(crate) static SECRETS_RULES: &[RuleDescriptor] = &[
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
