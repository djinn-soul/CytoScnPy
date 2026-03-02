use std::sync::OnceLock;

/// Returns rule IDs that should be augmented by taint analysis when available.
pub fn get_taint_sensitive_rules() -> &'static [&'static str] {
    static RULES: OnceLock<Vec<&'static str>> = OnceLock::new();
    RULES.get_or_init(|| {
        vec![
            crate::rules::ids::RULE_ID_SQL_INJECTION,
            crate::rules::ids::RULE_ID_SQL_RAW,
            crate::rules::ids::RULE_ID_SSRF,
            crate::rules::ids::RULE_ID_PATH_TRAVERSAL,
        ]
    })
}
