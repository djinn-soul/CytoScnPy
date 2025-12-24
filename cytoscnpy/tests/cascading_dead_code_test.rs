//! TDD tests for cascading dead code detection.
//!
//! Tests that methods inside unused classes are also flagged as unused.
//! This implements the "cascading deadness" feature request.
#![allow(clippy::unwrap_used, clippy::uninlined_format_args)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

// ============================================================================
// TDD RED PHASE: These tests define the expected behavior
// ============================================================================

/// Test that ALL methods (not just recursive ones) in an unused class are flagged.
/// This is the core test for cascading dead code detection.
#[test]
fn test_all_methods_in_unused_class_flagged() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("unused_class_methods.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
class UnusedClass:
    def method_a(self):
        return "a"
    
    def method_b(self):
        return "b"
    
    def method_c(self):
        # This method calls method_a internally
        return self.method_a() + "c"

# Class is never instantiated or used
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

    // ALL methods should be flagged, not just recursive ones
    let unused_method_names: Vec<String> = result
        .unused_methods
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    assert!(
        unused_method_names.contains(&"method_a".to_owned()),
        "method_a should be flagged (class is unused). Found: {:?}",
        unused_method_names
    );
    assert!(
        unused_method_names.contains(&"method_b".to_owned()),
        "method_b should be flagged (class is unused). Found: {:?}",
        unused_method_names
    );
    assert!(
        unused_method_names.contains(&"method_c".to_owned()),
        "method_c should be flagged (class is unused). Found: {:?}",
        unused_method_names
    );
}

/// Test that static methods in unused classes are also flagged.
#[test]
fn test_static_methods_in_unused_class_flagged() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("static_methods.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
class UnusedUtility:
    @staticmethod
    def helper_static():
        return "static helper"
    
    @classmethod
    def helper_classmethod(cls):
        return "classmethod helper"
    
    def regular_method(self):
        return "regular"

# UnusedUtility is never used
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let unused_class_names: Vec<String> = result
        .unused_classes
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();
    assert!(
        unused_class_names.contains(&"UnusedUtility".to_owned()),
        "UnusedUtility should be flagged as unused"
    );

    let unused_method_names: Vec<String> = result
        .unused_methods
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    // All methods including static/classmethod should be flagged
    assert!(
        unused_method_names.contains(&"helper_static".to_owned()),
        "helper_static should be flagged. Found: {:?}",
        unused_method_names
    );
    assert!(
        unused_method_names.contains(&"helper_classmethod".to_owned()),
        "helper_classmethod should be flagged. Found: {:?}",
        unused_method_names
    );
    assert!(
        unused_method_names.contains(&"regular_method".to_owned()),
        "regular_method should be flagged. Found: {:?}",
        unused_method_names
    );
}

/// Test that methods in USED classes are NOT flagged (regression check).
#[test]
fn test_methods_in_used_class_not_flagged() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("used_class.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
class UsedClass:
    def used_method(self):
        return "used"
    
    def internal_helper(self):
        return "helper"

# UsedClass is instantiated and used
obj = UsedClass()
result = obj.used_method()
print(result)
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    // UsedClass should NOT be flagged
    let unused_class_names: Vec<String> = result
        .unused_classes
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();
    assert!(
        !unused_class_names.contains(&"UsedClass".to_owned()),
        "UsedClass should NOT be flagged as unused"
    );

    // used_method should NOT be flagged (it was called)
    let unused_method_names: Vec<String> = result
        .unused_methods
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();
    assert!(
        !unused_method_names.contains(&"used_method".to_owned()),
        "used_method should NOT be flagged as unused"
    );
}

/// Test analyze_code (single file) also performs cascading detection.
#[test]
fn test_analyze_code_cascading_dead_code() {
    let code = r#"
class DeadClass:
    def dead_method_1(self):
        return 1
    
    def dead_method_2(self):
        return 2

# DeadClass is never used
"#;

    let analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze_code(code, std::path::Path::new("test.py"));

    let unused_class_names: Vec<String> = result
        .unused_classes
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();
    assert!(
        unused_class_names.contains(&"DeadClass".to_owned()),
        "DeadClass should be flagged"
    );

    let unused_method_names: Vec<String> = result
        .unused_methods
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();
    assert!(
        unused_method_names.contains(&"dead_method_1".to_owned()),
        "dead_method_1 should be flagged. Found: {:?}",
        unused_method_names
    );
    assert!(
        unused_method_names.contains(&"dead_method_2".to_owned()),
        "dead_method_2 should be flagged. Found: {:?}",
        unused_method_names
    );
}
