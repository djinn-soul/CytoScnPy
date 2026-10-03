//! Unit tests for naming style classification, AST scanning, and consistency reporting.

use std::path::{Path, PathBuf};

use super::classifier::{classify_identifier, is_dunder_name};
use super::scanner::{analyze_naming_from_source, analyze_naming_functions};
use super::types::{NamingDistributionResult, NamingStyle};
use crate::searchability::functions::FunctionDefinition;

fn make_fn(name: &str, file: &str, line: usize) -> FunctionDefinition {
    FunctionDefinition {
        name: name.to_owned(),
        file: PathBuf::from(file),
        line,
    }
}

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

#[test]
fn test_all_snake_case_functions() {
    let funcs = vec![
        make_fn("get_user", "test.py", 1),
        make_fn("_private_proc", "test.py", 5),
        make_fn("calculate_total", "test.py", 10),
    ];
    let res = analyze_naming_functions(&funcs);
    assert_eq!(res.stats.total_identifiers, 3);
    assert_eq!(res.stats.snake_case_count, 3);
    assert_eq!(res.stats.dominant_style, NamingStyle::SnakeCase);
    assert!((res.stats.dominant_style_ratio - 1.0).abs() < f64::EPSILON);
    assert!(res.outliers.is_empty());
    assert!(res.is_consistent(0.90));
}

#[test]
fn test_naming_with_outliers_and_dunders() {
    let funcs = vec![
        make_fn("__init__", "app.py", 1),
        make_fn("clean_data", "app.py", 10),
        make_fn("process_records", "app.py", 20),
        make_fn("save_all", "app.py", 30),
        make_fn("badCamelMethod", "app.py", 40),
        make_fn("Invalid_Mixed", "app.py", 50),
    ];
    let res = analyze_naming_functions(&funcs);
    assert_eq!(res.stats.dunder_count, 1);
    assert_eq!(res.stats.total_identifiers, 5);
    assert_eq!(res.stats.snake_case_count, 3);
    assert_eq!(res.stats.camel_case_count, 1);
    assert_eq!(res.stats.mixed_count, 1);
    assert_eq!(res.stats.dominant_style, NamingStyle::SnakeCase);
    assert_eq!(res.outliers.len(), 2);

    assert_eq!(res.outliers[0].name, "badCamelMethod");
    assert_eq!(res.outliers[0].detected_style, NamingStyle::CamelCase);
    assert_eq!(res.outliers[0].expected_style, NamingStyle::SnakeCase);

    assert_eq!(res.outliers[1].name, "Invalid_Mixed");
    assert_eq!(res.outliers[1].detected_style, NamingStyle::Mixed);
    assert_eq!(res.outliers[1].expected_style, NamingStyle::SnakeCase);

    // Consistency ratio: 3 / 5 = 0.6
    assert!((res.stats.dominant_style_ratio - 0.6).abs() < 1e-6);
    assert!(!res.is_consistent(0.85));
}

#[test]
fn test_analyze_from_source() {
    let code = r"
class Service:
    def __init__(self, name):
        self.name = name

    def fetch_user(self, user_id):
        return None

    def _internal_helper(self):
        return True

    def calculatePoints(self):
        return 42
";
    let res = analyze_naming_from_source(code, Path::new("service.py"));
    assert_eq!(res.stats.dunder_count, 1);
    assert_eq!(res.stats.total_identifiers, 4);
    assert_eq!(res.stats.snake_case_count, 2);
    assert_eq!(res.stats.pascal_case_count, 1);
    assert_eq!(res.stats.camel_case_count, 1);
    assert_eq!(res.outliers.len(), 1);
    assert_eq!(res.outliers[0].name, "calculatePoints");
}

#[test]
fn test_empty_input() {
    let res = analyze_naming_functions(&[]);
    assert_eq!(res.stats.total_identifiers, 0);
    assert_eq!(res.stats.dominant_style, NamingStyle::SnakeCase);
    assert!((res.stats.dominant_style_ratio - 1.0).abs() < f64::EPSILON);
    assert!(res.outliers.is_empty());
    assert!(res.is_consistent(0.80));
}

#[test]
fn test_reporter_formatting() {
    let funcs = vec![
        make_fn("valid_one", "mod.py", 1),
        make_fn("invalidTwo", "mod.py", 5),
    ];
    let res = analyze_naming_functions(&funcs);

    let mut term_output = Vec::new();
    super::reporter::print_terminal_report(&res, None, &mut term_output).unwrap();
    let term_str = String::from_utf8(term_output).unwrap();
    assert!(term_str.contains("Naming Style Distribution"));
    assert!(term_str.contains("snake_case"));
    assert!(term_str.contains("invalidTwo"));

    let mut json_output = Vec::new();
    super::reporter::print_json_report(&res, &mut json_output).unwrap();
    let json_str = String::from_utf8(json_output).unwrap();
    let parsed: NamingDistributionResult = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed.stats.total_identifiers, 2);
    assert_eq!(parsed.outliers.len(), 1);
}

#[test]
fn test_unittest_fixtures_exempted() {
    let funcs = vec![
        make_fn("setUp", "test_mod.py", 1),
        make_fn("tearDown", "test_mod.py", 5),
        make_fn("test_feature", "test_mod.py", 10),
    ];
    let res = analyze_naming_functions(&funcs);
    assert_eq!(res.stats.total_identifiers, 1);
    assert_eq!(res.stats.snake_case_count, 1);
    assert!(res.outliers.is_empty());
}

#[test]
fn test_mixed_only_does_not_become_dominant() {
    let funcs = vec![
        make_fn("bad_MethodOne", "app.py", 1),
        make_fn("another_BadTwo", "app.py", 5),
    ];
    let res = analyze_naming_functions(&funcs);
    assert_eq!(res.stats.dominant_style, NamingStyle::SnakeCase);
    assert_eq!(res.stats.total_identifiers, 2);
    assert_eq!(res.stats.mixed_count, 2);
    assert_eq!(res.stats.snake_case_count, 0);
    assert!(res.stats.dominant_style_ratio.abs() < f64::EPSILON);
    assert!(res.stats.consistency_score().abs() < f64::EPSILON);
    assert_eq!(res.outliers.len(), 2);
    assert!(!res.is_consistent(0.5));
}

#[test]
fn test_normalize_consistency_threshold() {
    assert!((super::types::normalize_consistency_threshold(0.85) - 0.85).abs() < f64::EPSILON);
    assert!((super::types::normalize_consistency_threshold(85.0) - 0.85).abs() < f64::EPSILON);
    assert!((super::types::normalize_consistency_threshold(100.0) - 1.0).abs() < f64::EPSILON);
    assert!((super::types::normalize_consistency_threshold(1.0) - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_class_naming_conforming_and_outliers() {
    let code = "class GoodClass:\n    pass\n\nclass bad_class:\n    pass\n";
    let res = analyze_naming_from_source(code, Path::new("test.py"));
    assert_eq!(res.stats.total_identifiers, 2);
    assert_eq!(res.stats.pascal_case_count, 1);
    assert_eq!(res.stats.snake_case_count, 1);
    assert_eq!(res.outliers.len(), 1);
    assert_eq!(res.outliers[0].name, "bad_class");
    assert_eq!(res.outliers[0].detected_style, NamingStyle::SnakeCase);
    assert_eq!(res.outliers[0].expected_style, NamingStyle::PascalCase);
}

#[test]
fn test_classes_do_not_skew_dominant_function_style() {
    let code = "class ClassOne:\n    pass\nclass ClassTwo:\n    pass\nclass ClassThree:\n    pass\ndef helper_func():\n    pass\n";
    let res = analyze_naming_from_source(code, Path::new("test.py"));
    assert_eq!(res.stats.dominant_style, NamingStyle::SnakeCase);
    assert!(res.outliers.is_empty());
    assert!((res.stats.dominant_style_ratio - 1.0).abs() < f64::EPSILON);
}
