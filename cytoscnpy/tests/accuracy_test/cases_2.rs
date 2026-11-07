use super::*;

// ============================================================================
// Regression Tests - Ensure existing functionality is preserved
// ============================================================================

/// Regression test: Ensure decorator patterns still work.
#[test]
fn test_decorator_wrapper_still_works() {
    let dir = project_tempdir();
    let file_path = dir.path().join("decorator.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
def decorator(func):
    def wrapper(*args, **kwargs):
        print("Before")
        result = func(*args, **kwargs)
        print("After")
        return result
    return wrapper

@decorator
def decorated_func():
    pass

decorated_func()
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let unused_func_names: Vec<String> = result
        .unused_functions
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    // wrapper is returned, should NOT be flagged
    assert!(
        !unused_func_names.contains(&"wrapper".to_owned()),
        "wrapper should not be flagged as it is returned"
    );
}

/// Regression test: __all__ exports still work.
#[test]
fn test_all_exports_regression() {
    let dir = project_tempdir();
    let file_path = dir.path().join("exports.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
__all__ = ["exported_func"]

def exported_func():
    pass

def not_exported_func():
    pass
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let unused_func_names: Vec<String> = result
        .unused_functions
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    // exported_func is in __all__, should NOT be flagged
    assert!(
        !unused_func_names.contains(&"exported_func".to_owned()),
        "exported_func is in __all__ and should not be flagged"
    );

    // not_exported_func is not in __all__ and not called, SHOULD be flagged
    assert!(
        unused_func_names.contains(&"not_exported_func".to_owned()),
        "not_exported_func should be flagged"
    );
}
