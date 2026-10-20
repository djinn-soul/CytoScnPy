//! Unit tests for naming style classification.

use super::super::classifier::{classify_identifier, is_dunder_name};
use super::super::types::NamingStyle;

#[test]
fn test_classify_snake_case() {
    assert_eq!(
        classify_identifier("get_user"),
        Some(NamingStyle::SnakeCase)
    );
    assert_eq!(
        classify_identifier("process_data_123"),
        Some(NamingStyle::SnakeCase)
    );
    assert_eq!(classify_identifier("item"), Some(NamingStyle::SnakeCase));
    assert_eq!(
        classify_identifier("_private_helper"),
        Some(NamingStyle::SnakeCase)
    );
    assert_eq!(
        classify_identifier("__mangled_method"),
        Some(NamingStyle::SnakeCase)
    );
    assert_eq!(classify_identifier("class_"), Some(NamingStyle::SnakeCase));
    assert_eq!(classify_identifier("_"), Some(NamingStyle::SnakeCase));
}

#[test]
fn test_classify_camel_case() {
    assert_eq!(classify_identifier("getUser"), Some(NamingStyle::CamelCase));
    assert_eq!(
        classify_identifier("calculateTotal"),
        Some(NamingStyle::CamelCase)
    );
    assert_eq!(
        classify_identifier("_fetchData"),
        Some(NamingStyle::CamelCase)
    );
    assert_eq!(
        classify_identifier("__asyncFetch"),
        Some(NamingStyle::CamelCase)
    );
}

#[test]
fn test_classify_pascal_case() {
    assert_eq!(
        classify_identifier("UserFactory"),
        Some(NamingStyle::PascalCase)
    );
    assert_eq!(
        classify_identifier("ResponseBuilder"),
        Some(NamingStyle::PascalCase)
    );
    assert_eq!(
        classify_identifier("_SpecialHandler"),
        Some(NamingStyle::PascalCase)
    );
}

#[test]
fn test_classify_screaming_snake() {
    assert_eq!(
        classify_identifier("MAX_SIZE"),
        Some(NamingStyle::ScreamingSnakeCase)
    );
    assert_eq!(
        classify_identifier("DEFAULT_TIMEOUT"),
        Some(NamingStyle::ScreamingSnakeCase)
    );
    assert_eq!(
        classify_identifier("_BUFFER_LEN"),
        Some(NamingStyle::ScreamingSnakeCase)
    );
    assert_eq!(
        classify_identifier("PI"),
        Some(NamingStyle::ScreamingSnakeCase)
    );
}

#[test]
fn test_classify_mixed() {
    assert_eq!(classify_identifier("get_User"), Some(NamingStyle::Mixed));
    assert_eq!(
        classify_identifier("some_camelCase"),
        Some(NamingStyle::Mixed)
    );
    assert_eq!(classify_identifier("FOO_bar"), Some(NamingStyle::Mixed));
}

#[test]
fn test_dunder_exemptions() {
    assert!(is_dunder_name("__init__"));
    assert!(is_dunder_name("__repr__"));
    assert!(is_dunder_name("__str__"));
    assert!(is_dunder_name("__enter__"));
    assert!(is_dunder_name("__exit__"));
    assert!(!is_dunder_name("__mangled_only"));
    assert!(!is_dunder_name("regular_name"));
    assert!(!is_dunder_name("____"));
    assert!(!is_dunder_name("_____"));

    // classify_identifier returns None for dunder protocols
    assert_eq!(classify_identifier("__init__"), None);
    assert_eq!(classify_identifier("__repr__"), None);
}

#[test]
fn test_anonymous_and_empty() {
    assert_eq!(classify_identifier(""), None);
    assert_eq!(classify_identifier("<anonymous>"), None);
    assert_eq!(classify_identifier("<lambda>"), None);
}
