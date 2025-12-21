//! TDD tests for F1 score and accuracy improvements.
#![allow(
    clippy::unwrap_used,
    clippy::uninlined_format_args,
    clippy::needless_raw_string_hashes
)]
//!
//! These tests define expected behavior for:
//! 1. Class-method linking (methods in unused classes should be flagged)
//! 2. Nested function call tracking (nested functions called within parent scope)
//! 3. Pattern matching variable usage

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

// ============================================================================
// PRIORITY 1: Class-Method Linking
// When a class is unused, its methods should also be detected as unused.
// ============================================================================

/// Test that methods inside an unused class are also flagged as unused.
/// This is a key accuracy improvement to reduce false negatives.
#[test]
fn test_unused_class_methods_are_detected() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("class_methods.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
class UsedClass:
    def used_method(self):
        return "used"

class UnusedClass:
    def method_a(self):
        return "a"
    
    def method_b(self):
        return "b"

# Only UsedClass is used
obj = UsedClass()
obj.used_method()
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    // UnusedClass should be flagged
    let unused_class_names: Vec<String> = result
        .unused_classes
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    assert!(
        unused_class_names.contains(&"UnusedClass".to_owned()),
        "UnusedClass should be flagged as unused"
    );

    // Methods of UnusedClass should also be flagged
    let unused_method_names: Vec<String> = result
        .unused_methods
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    assert!(
        unused_method_names.contains(&"method_a".to_owned()),
        "method_a in UnusedClass should be flagged as unused. Found: {:?}",
        unused_method_names
    );
    assert!(
        unused_method_names.contains(&"method_b".to_owned()),
        "method_b in UnusedClass should be flagged as unused. Found: {:?}",
        unused_method_names
    );

    // Methods of UsedClass should NOT be flagged
    assert!(
        !unused_method_names.contains(&"used_method".to_owned()),
        "used_method in UsedClass should NOT be flagged"
    );
}

/// Test that recursive methods in unused classes are also flagged.
/// Even if a method calls itself, if the class is unused, the method is unused.
#[test]
fn test_recursive_method_in_unused_class_is_flagged() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("recursive_class.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
class UnusedClass:
    def recursive_method(self, n):
        if n <= 1:
            return 1
        return n * self.recursive_method(n - 1)

# UnusedClass is never instantiated
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let unused_method_names: Vec<String> = result
        .unused_methods
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    // recursive_method should be flagged because the class is unused
    // even though it has a self-reference
    assert!(
        unused_method_names.contains(&"recursive_method".to_owned()),
        "recursive_method should be flagged because UnusedClass is unused. Found: {:?}",
        unused_method_names
    );
}

// ============================================================================
// PRIORITY 2: Nested Function Call Tracking
// Nested functions called within the parent function should be marked as used.
// ============================================================================

/// Test that nested functions called within the same parent scope are NOT flagged.
/// This fixes false positives like `used_inner @ code.py:4`.
#[test]
fn test_nested_function_call_within_parent_scope() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("nested_calls.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
def outer_function():
    def used_inner():
        return "Used inner function"

    def unused_inner():
        return "Unused inner function"

    # Only used_inner is called
    result = used_inner()
    return result

# Call outer_function
print(outer_function())
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

    // used_inner is called, should NOT be flagged
    assert!(
        !unused_func_names.contains(&"used_inner".to_owned()),
        "used_inner is called within outer_function and should NOT be flagged. Found unused: {:?}",
        unused_func_names
    );

    // unused_inner is NOT called, SHOULD be flagged
    assert!(
        unused_func_names.contains(&"unused_inner".to_owned()),
        "unused_inner is not called and SHOULD be flagged as unused. Found: {:?}",
        unused_func_names
    );

    // outer_function is called at module level, should NOT be flagged
    assert!(
        !unused_func_names.contains(&"outer_function".to_owned()),
        "outer_function is called and should NOT be flagged"
    );
}

/// Test that functions returned from factory functions are marked as used.
#[test]
fn test_returned_nested_function_is_used() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("factory.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
def factory():
    def inner_returned():
        return "I am returned"

    def inner_unused():
        return "I am not used"

    return inner_returned

# Factory returns inner_returned which is then called
func = factory()
func()
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

    // inner_returned is returned, should NOT be flagged
    assert!(
        !unused_func_names.contains(&"inner_returned".to_owned()),
        "inner_returned is returned from factory and should NOT be flagged. Found: {:?}",
        unused_func_names
    );

    // inner_unused is neither called nor returned, SHOULD be flagged
    assert!(
        unused_func_names.contains(&"inner_unused".to_owned()),
        "inner_unused is not used and SHOULD be flagged. Found: {:?}",
        unused_func_names
    );
}

// ============================================================================
// PRIORITY 3: Pattern Matching Variable Usage
// Variables bound in match patterns and used in the case body should be used.
// ============================================================================

/// Test that variables bound in match patterns and used are NOT flagged.
#[test]
fn test_pattern_matching_bound_variable_used() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("pattern_match.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
def handle_command(command):
    match command:
        case ["load", filename]:
            # filename is bound and USED
            print(f"Loading {{filename}}")
        case ["save", target]:
            # target is bound but NOT used (should be flagged)
            print("Saving...")
        case {{"x": x_val, "y": y_val}}:
            # x_val and y_val are bound and USED
            print(f"Point at {{x_val}}, {{y_val}}")
        case _:
            print("Unknown")

handle_command(["load", "test.txt"])
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let unused_var_names: Vec<String> = result
        .unused_variables
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    // filename is used in print, should NOT be flagged
    assert!(
        !unused_var_names.contains(&"filename".to_owned()),
        "filename is used in print and should NOT be flagged. Found: {:?}",
        unused_var_names
    );

    // x_val and y_val are used in print, should NOT be flagged
    assert!(
        !unused_var_names.contains(&"x_val".to_owned()),
        "x_val is used in print and should NOT be flagged"
    );
    assert!(
        !unused_var_names.contains(&"y_val".to_owned()),
        "y_val is used in print and should NOT be flagged"
    );

    // target is NOT used, SHOULD be flagged
    assert!(
        unused_var_names.contains(&"target".to_owned()),
        "target is not used and SHOULD be flagged. Found: {:?}",
        unused_var_names
    );
}

// ============================================================================
// Regression Tests - Ensure existing functionality is preserved
// ============================================================================

/// Regression test: Ensure decorator patterns still work.
#[test]
fn test_decorator_wrapper_still_works() {
    let dir = tempdir().unwrap();
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
    let dir = tempdir().unwrap();
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
