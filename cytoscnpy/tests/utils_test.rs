//! Tests for utility functions.
//!
//! Covers pragma detection, test path identification, and framework path helpers.
use cytoscnpy::utils::{get_ignored_lines, is_framework_path, is_test_path};

#[test]
fn test_pragma_detection() {
    let source = r#"
def used_function():
    return 42

def unused_function():  # pragma: no cytoscnpy
    return "ignored"

class MyClass:  # pragma: no cytoscnpy
    pass
"#;
    let ignored = get_ignored_lines(source);

    // Lines 5 and 8 should be ignored (1-indexed)
    assert!(ignored.contains_key(&5), "Should detect pragma on line 5");
    assert!(ignored.contains_key(&8), "Should detect pragma on line 8");
    assert_eq!(ignored.len(), 2, "Should find exactly 2 pragma lines");
}

#[test]
fn test_no_pragmas() {
    let source = r"
def regular_function():
    return 42
";
    let ignored = get_ignored_lines(source);
    assert_eq!(ignored.len(), 0, "Should find no pragma lines");
}

#[test]
fn test_is_test_path() {
    assert!(is_test_path("tests/test_foo.py"));
    assert!(is_test_path("tests/foo_test.py"));
    assert!(is_test_path("project/tests/test_bar.py"));
    assert!(is_test_path("test_main.py"));
    assert!(is_test_path("my_test.py"));

    // Windows paths
    assert!(is_test_path("tests\\test_foo.py"));
    assert!(is_test_path("project\\tests\\test_bar.py"));

    // Negative cases
    assert!(!is_test_path("main.py"));
    assert!(!is_test_path("utils.py"));
    // "tests/utils.py" matches "tests/" prefix. So it IS a test path.
    assert!(is_test_path("tests/utils.py"));

    assert!(!is_test_path("prod_code.py"));
}

#[test]
fn test_is_framework_path() {
    assert!(is_framework_path("views.py"));
    assert!(is_framework_path("api/views.py"));
    assert!(is_framework_path("handlers.py"));
    assert!(is_framework_path("routes.py"));
    assert!(is_framework_path("endpoints.py"));
    assert!(is_framework_path("api.py"));

    // Case insensitivity
    assert!(is_framework_path("Views.py"));

    // Negative cases
    assert!(!is_framework_path("main.py"));
    assert!(!is_framework_path("utils.py"));
    assert!(!is_framework_path("models.py")); // models.py is not in the default list
}
