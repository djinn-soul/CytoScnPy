//! Python-aware identifier casing classification and exemption rules.

use super::types::NamingStyle;

/// Checks if an identifier is a Python special dunder method/attribute (e.g. `__init__`).
#[must_use]
pub fn is_dunder_name(name: &str) -> bool {
    name.len() >= 5
        && name.starts_with("__")
        && name.ends_with("__")
        && !name.trim_matches('_').is_empty()
}

/// Checks if an identifier is an anonymous placeholder or empty.
#[must_use]
pub fn is_anonymous_or_empty(name: &str) -> bool {
    name.is_empty() || name == "<anonymous>" || name == "<lambda>"
}

/// Checks if an identifier string conforms to `snake_case`.
fn is_snake_case(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    if s.ends_with('_') || s.contains("__") {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Checks if an identifier string conforms to `SCREAMING_SNAKE_CASE`.
fn is_screaming_snake_case(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_uppercase() {
        return false;
    }
    if s.ends_with('_') || s.contains("__") {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Checks if an identifier string conforms to `PascalCase`.
fn is_pascal_case(s: &str) -> bool {
    if s.contains('_') {
        return false;
    }
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_uppercase()
        && s.chars().any(|c| c.is_ascii_lowercase())
        && s.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Checks if an identifier string conforms to `camelCase`.
fn is_camel_case(s: &str) -> bool {
    if s.contains('_') {
        return false;
    }
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && s.chars().any(|c| c.is_ascii_uppercase())
        && s.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Classifies a Python identifier into its detected naming style.
///
/// Python-aware handling:
/// - Strips single or double leading underscores (`_private`, `__mangled`).
/// - Strips single trailing underscores used to avoid keyword collision (`class_`, `id_`).
/// - Single dummy variable names (`_` or `__`) classify as `snake_case`.
/// - Dunder names (`__init__`) are recognized as language protocols and should be handled
///   via `is_dunder_name` before general classification.
#[must_use]
pub fn classify_identifier(name: &str) -> Option<NamingStyle> {
    if is_anonymous_or_empty(name) {
        return None;
    }

    if is_dunder_name(name) {
        return None;
    }

    // Strip leading private/mangled underscores (PEP 8)
    let without_leading = name.trim_start_matches('_');
    if without_leading.is_empty() {
        return Some(NamingStyle::SnakeCase);
    }

    // Strip trailing underscore used to avoid keyword collision (PEP 8)
    let clean = without_leading.trim_end_matches('_');
    if clean.is_empty() {
        return Some(NamingStyle::SnakeCase);
    }

    if is_snake_case(clean) {
        return Some(NamingStyle::SnakeCase);
    }

    if is_screaming_snake_case(clean) {
        return Some(NamingStyle::ScreamingSnakeCase);
    }

    if is_pascal_case(clean) {
        return Some(NamingStyle::PascalCase);
    }

    if is_camel_case(clean) {
        return Some(NamingStyle::CamelCase);
    }

    Some(NamingStyle::Mixed)
}

/// Known standard framework lifecycle / test fixtures using `camelCase` in Python standard library.
pub const UNITTEST_FIXTURE_NAMES: &[&str] = &[
    "setUp",
    "tearDown",
    "setUpClass",
    "tearDownClass",
    "setUpModule",
    "tearDownModule",
    "assertCountEqual",
    "assertRaises",
    "assertEqual",
    "assertNotEqual",
    "assertTrue",
    "assertFalse",
    "assertIsNone",
    "assertIsNotNone",
    "assertIn",
    "assertNotIn",
    "assertIsInstance",
    "addCleanup",
    "doCleanups",
];

/// Checks if an identifier is a standard `unittest` fixture method.
#[must_use]
pub fn is_unittest_fixture(name: &str) -> bool {
    UNITTEST_FIXTURE_NAMES.contains(&name)
}
