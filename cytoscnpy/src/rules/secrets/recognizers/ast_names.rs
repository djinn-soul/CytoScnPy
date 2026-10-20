//! Suspicious variable-name matching for the AST secret recognizer.

use super::super::types::is_test_name;

/// Suspicious variable name patterns.
const SUSPICIOUS_NAMES: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "secret",
    "key",
    "token",
    "auth",
    "credential",
    "api_key",
    "apikey",
    "private_key",
    "access_token",
    "secret_key",
    "auth_token",
    "bearer",
    "client_secret",
    "app_secret",
    "encryption_key",
    "signing_key",
    "master_key",
];

/// Check if a name matches suspicious patterns.
pub(super) fn matches_suspicious_name(name: &str, custom_names: &[String]) -> bool {
    let lower = name.to_lowercase();

    // 1. Check exclusions (Safe patterns)
    const SAFE_NAME_SUBSTRINGS: &[&str] = &[
        "keyboard",
        "keyword",
        "monkey",
        "donkey",
        "tracking_id",
        "uuid",
        "public",
        "example",
        "sample",
    ];

    if SAFE_NAME_SUBSTRINGS.iter().any(|&s| lower.contains(s))
        || is_test_name(&lower)
        || lower.ends_with("_regex")
        || lower.ends_with("_pattern")
        || lower.ends_with("_re")
        || lower.ends_with("_fmt")
        || lower.ends_with("_format")
    {
        return false;
    }

    // jwt_token-style vars hold bearer tokens, not secrets — CSP-S122 covers jwt_secret/jwt_key
    if lower.contains("jwt") && lower.contains("token") {
        return false;
    }

    // 2. Check built-in patterns with word boundary awareness
    for &pattern in SUSPICIOUS_NAMES {
        // Use match_indices for more idiomatic and efficient matching
        for (absolute_idx, _) in lower.match_indices(pattern) {
            // For short or common keywords, enforce word boundaries
            // to avoid matching 'keyboard', 'monkey', 'donkey', etc.
            if matches!(pattern, "key" | "pwd" | "auth" | "token") {
                // Check if it's a standalone word or part of a snake_case/camelCase identifier
                let before = if absolute_idx > 0 {
                    lower.as_bytes().get(absolute_idx - 1).map(|&b| b as char)
                } else {
                    None
                };
                let after = lower
                    .as_bytes()
                    .get(absolute_idx + pattern.len())
                    .map(|&b| b as char);

                let boundary_before = before.map_or(true, |c| !c.is_alphanumeric());
                let boundary_after = after.map_or(true, |c| !c.is_alphanumeric());
                let camel_boundary_before = is_camel_boundary_before(name, absolute_idx);
                let camel_boundary_after =
                    is_camel_boundary_after(name, absolute_idx + pattern.len());

                if (boundary_before || camel_boundary_before)
                    && (boundary_after || camel_boundary_after)
                {
                    return true;
                }
            } else {
                // Not a length-sensitive keyword, any match is suspicious
                return true;
            }
        }
    }

    // 3. Check custom patterns
    custom_names
        .iter()
        .any(|s| lower.contains(&s.to_lowercase()))
}

fn is_camel_boundary_before(name: &str, start: usize) -> bool {
    if !name.is_ascii() || start == 0 {
        return false;
    }
    let bytes = name.as_bytes();
    let current = bytes[start] as char;
    current.is_ascii_uppercase()
}

fn is_camel_boundary_after(name: &str, end: usize) -> bool {
    if !name.is_ascii() || end >= name.len() || end == 0 {
        return false;
    }
    let bytes = name.as_bytes();
    let before = bytes[end - 1] as char;
    let after = bytes[end] as char;
    before.is_ascii_lowercase() && after.is_ascii_uppercase()
}
