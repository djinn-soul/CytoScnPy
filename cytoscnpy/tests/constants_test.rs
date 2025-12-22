//! Comprehensive tests for pattern matching constants
//! Tests for constant detection and analysis.
#![allow(clippy::explicit_iter_loop)]
#![allow(clippy::uninlined_format_args)]

use cytoscnpy::constants::{
    get_auto_called, get_default_exclude_folders, get_framework_file_re, get_penalties,
    get_test_decor_re, get_test_file_re, get_test_import_re, get_test_method_pattern,
    get_unittest_lifecycle_methods,
};

// =============================================================================
// Test File Regex Tests
// =============================================================================

#[test]
fn test_test_file_patterns_match() {
    let re = get_test_file_re();

    // test_*.py patterns
    assert!(re.is_match("test_foo.py"), "Should match test_foo.py");
    assert!(re.is_match("test_bar.py"), "Should match test_bar.py");

    // *_test.py patterns
    assert!(re.is_match("module_test.py"), "Should match module_test.py");
    assert!(re.is_match("foo_test.py"), "Should match foo_test.py");

    // tests/ directory
    assert!(re.is_match("tests/something.py"), "Should match tests/ dir");
    assert!(
        re.is_match("project/tests/module.py"),
        "Should match nested tests/ dir"
    );

    // test/ directory
    assert!(re.is_match("test/something.py"), "Should match test/ dir");

    // conftest.py
    assert!(re.is_match("conftest.py"), "Should match conftest.py");
    assert!(
        re.is_match("tests/conftest.py"),
        "Should match tests/conftest.py"
    );
}

#[test]
fn test_non_test_file_patterns_dont_match() {
    let re = get_test_file_re();

    assert!(!re.is_match("main.py"), "Should not match main.py");
    assert!(!re.is_match("utils.py"), "Should not match utils.py");
    assert!(
        !re.is_match("testing_utils.py"),
        "Should not match testing_utils.py"
    );
    assert!(
        !re.is_match("mytest.py"),
        "Should not match mytest.py (no underscore)"
    );
    assert!(
        !re.is_match("testdata.py"),
        "Should not match testdata.py (no underscore)"
    );
}

// =============================================================================
// Test Import Regex Tests
// =============================================================================

#[test]
fn test_test_import_patterns_match() {
    let re = get_test_import_re();

    assert!(re.is_match("pytest"), "Should match pytest");
    assert!(re.is_match("pytest.mark"), "Should match pytest.mark");
    assert!(re.is_match("unittest"), "Should match unittest");
    assert!(re.is_match("unittest.mock"), "Should match unittest.mock");
    assert!(re.is_match("mock"), "Should match mock");
    assert!(re.is_match("nose"), "Should match nose");
    assert!(re.is_match("responses"), "Should match responses");
}

#[test]
fn test_non_test_import_patterns_dont_match() {
    let re = get_test_import_re();

    assert!(!re.is_match("requests"), "Should not match requests");
    assert!(!re.is_match("flask"), "Should not match flask");
    assert!(!re.is_match("django"), "Should not match django");
    assert!(
        !re.is_match("mocking_library"),
        "Should not match mocking_library"
    );
}

// =============================================================================
// Test Decorator Regex Tests
// =============================================================================

#[test]
fn test_test_decorator_patterns_match() {
    let re = get_test_decor_re();

    assert!(re.is_match("pytest.fixture"), "Should match pytest.fixture");
    assert!(re.is_match("pytest.mark"), "Should match pytest.mark");
    assert!(re.is_match("patch"), "Should match patch");
    assert!(
        re.is_match("responses.activate"),
        "Should match responses.activate"
    );
    assert!(re.is_match("freeze_time"), "Should match freeze_time");
}

#[test]
fn test_non_test_decorator_patterns_dont_match() {
    let re = get_test_decor_re();

    assert!(!re.is_match("property"), "Should not match property");
    assert!(
        !re.is_match("staticmethod"),
        "Should not match staticmethod"
    );
    assert!(!re.is_match("classmethod"), "Should not match classmethod");
    assert!(!re.is_match("dataclass"), "Should not match dataclass");
}

// =============================================================================
// Test Method Pattern Tests
// =============================================================================

#[test]
fn test_test_method_pattern_matches() {
    let re = get_test_method_pattern();

    assert!(re.is_match("test_something"), "Should match test_something");
    assert!(re.is_match("test_foo_bar"), "Should match test_foo_bar");
    assert!(re.is_match("test_1"), "Should match test_1");
}

#[test]
fn test_test_method_pattern_doesnt_match() {
    let re = get_test_method_pattern();

    assert!(
        !re.is_match("testing_something"),
        "Should not match testing_something"
    );
    assert!(
        !re.is_match("testfoo"),
        "Should not match testfoo (no underscore)"
    );
    assert!(
        !re.is_match("my_test_helper"),
        "Should not match my_test_helper"
    );
    assert!(!re.is_match("setup"), "Should not match setup");
}

// =============================================================================
// Framework File Regex Tests
// =============================================================================

#[test]
fn test_framework_file_patterns_match() {
    let re = get_framework_file_re();

    assert!(re.is_match("views.py"), "Should match views.py");
    assert!(re.is_match("handlers.py"), "Should match handlers.py");
    assert!(re.is_match("api.py"), "Should match api.py");
    assert!(re.is_match("routes.py"), "Should match routes.py");
    assert!(re.is_match("endpoints.py"), "Should match endpoints.py");
    assert!(re.is_match("urls.py"), "Should match urls.py");

    // With paths
    assert!(re.is_match("app/views.py"), "Should match app/views.py");
    assert!(
        re.is_match("project/api/handlers.py"),
        "Should match nested path"
    );
}

#[test]
fn test_non_framework_file_patterns_dont_match() {
    let re = get_framework_file_re();

    assert!(!re.is_match("models.py"), "Should not match models.py");
    assert!(!re.is_match("main.py"), "Should not match main.py");
    assert!(!re.is_match("utils.py"), "Should not match utils.py");
    assert!(!re.is_match("services.py"), "Should not match services.py");
}

// =============================================================================
// Constants Validation Tests
// =============================================================================

#[test]
fn test_penalties_structure() {
    let penalties = get_penalties();

    assert!(
        penalties.contains_key("private_name"),
        "Should have private_name penalty"
    );
    assert!(
        penalties.contains_key("dunder_or_magic"),
        "Should have dunder_or_magic penalty"
    );
    assert!(
        penalties.contains_key("test_related"),
        "Should have test_related penalty"
    );
    assert!(
        penalties.contains_key("framework_magic"),
        "Should have framework_magic penalty"
    );

    // Verify reasonable values (0-100)
    for (key, value) in penalties.iter() {
        assert!(*value <= 100, "Penalty {} should be <= 100", key);
    }
}

#[test]
fn test_auto_called_methods() {
    let auto_called = get_auto_called();

    assert!(auto_called.contains("__init__"), "Should contain __init__");
    assert!(
        auto_called.contains("__enter__"),
        "Should contain __enter__"
    );
    assert!(auto_called.contains("__exit__"), "Should contain __exit__");
}

#[test]
fn test_unittest_lifecycle_methods() {
    let lifecycle = get_unittest_lifecycle_methods();

    assert!(lifecycle.contains("setUp"), "Should contain setUp");
    assert!(lifecycle.contains("tearDown"), "Should contain tearDown");
    assert!(
        lifecycle.contains("setUpClass"),
        "Should contain setUpClass"
    );
    assert!(
        lifecycle.contains("tearDownClass"),
        "Should contain tearDownClass"
    );
    assert!(
        lifecycle.contains("setUpModule"),
        "Should contain setUpModule"
    );
    assert!(
        lifecycle.contains("tearDownModule"),
        "Should contain tearDownModule"
    );
}

#[test]
fn test_default_exclude_folders() {
    let exclude = get_default_exclude_folders();

    assert!(
        exclude.contains("__pycache__"),
        "Should exclude __pycache__"
    );
    assert!(exclude.contains(".git"), "Should exclude .git");
    assert!(
        exclude.contains(".pytest_cache"),
        "Should exclude .pytest_cache"
    );
    assert!(exclude.contains("venv"), "Should exclude venv");
    assert!(exclude.contains(".venv"), "Should exclude .venv");
    assert!(exclude.contains("build"), "Should exclude build");
    assert!(exclude.contains("dist"), "Should exclude dist");
}

// =============================================================================
// Regex Edge Cases Tests
// =============================================================================

#[test]
fn test_test_file_regex_path_separators() {
    let re = get_test_file_re();

    // Unix paths
    assert!(
        re.is_match("project/tests/test_foo.py"),
        "Should match Unix path"
    );

    // Windows paths
    assert!(
        re.is_match("project\\tests\\test_foo.py"),
        "Should match Windows path"
    );
    assert!(
        re.is_match("tests\\module.py"),
        "Should match Windows tests dir"
    );
}

#[test]
fn test_framework_file_regex_case_insensitivity() {
    let re = get_framework_file_re();

    // The regex has (?i) flag for case insensitivity
    assert!(re.is_match("Views.py"), "Should match Views.py (capital)");
    assert!(re.is_match("VIEWS.PY"), "Should match VIEWS.PY (uppercase)");
    assert!(re.is_match("Handlers.py"), "Should match Handlers.py");
}

#[test]
fn test_import_regex_with_deep_modules() {
    let re = get_test_import_re();

    assert!(
        re.is_match("pytest.mark.parametrize"),
        "Should match deep pytest module"
    );
    assert!(
        re.is_match("unittest.mock.patch"),
        "Should match deep unittest module"
    );
}

#[test]
fn test_conftest_detection() {
    let re = get_test_file_re();

    assert!(
        re.is_match("conftest.py"),
        "Should match conftest.py at root"
    );
    assert!(
        re.is_match("tests/conftest.py"),
        "Should match conftest.py in tests"
    );
    assert!(
        re.is_match("project/tests/conftest.py"),
        "Should match nested conftest.py"
    );
}
