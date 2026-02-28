use crate::rules::ids;
use crate::rules::RuleMetadata;

/// Rule metadata for assert usage.
pub const META_ASSERT: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_ASSERT,
    category: super::super::CAT_BEST_PRACTICES,
};
/// Rule metadata for insecure imports.
pub const META_INSECURE_IMPORT: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_INSECURE_IMPORT,
    category: super::super::CAT_BEST_PRACTICES,
};
/// Rule metadata for disabled Jinja2 autoescape.
pub const META_JINJA_AUTOESCAPE: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_JINJA_AUTOESCAPE,
    category: super::super::CAT_BEST_PRACTICES,
};
/// Rule metadata for blacklisted call usage.
pub const META_BLACKLIST: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_BLACKLIST,
    category: super::super::CAT_BEST_PRACTICES,
};
/// Rule metadata for logging sensitive data.
pub const META_LOGGING_SENSITIVE: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_LOGGING_SENSITIVE,
    category: super::super::CAT_PRIVACY,
};
/// Rule metadata for `input()` usage.
pub const META_INPUT: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_INPUT,
    category: super::super::CAT_CODE_EXEC,
};
