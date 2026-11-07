use super::*;

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
