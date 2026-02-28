pub(crate) mod danger;
pub(crate) mod quality;
pub(crate) mod secrets;

use super::types::{rule, RuleCategory, RuleDescriptor, RuleSeverity};

/// Documentation path for dangerous code rules.
pub const DOC_DANGEROUS_CODE: &str = "docs/dangerous-code.md";
/// Documentation path for quality rules.
pub const DOC_QUALITY: &str = "docs/quality.md";
/// Documentation path for secrets rules.
pub const DOC_SECURITY: &str = "docs/security.md";
