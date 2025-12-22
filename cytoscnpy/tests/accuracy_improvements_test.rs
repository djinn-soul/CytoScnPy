//! Tests for accuracy improvements (return tracking, `__all__` exports, `TYPE_CHECKING` imports).
#![allow(clippy::unwrap_used)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::doc_markdown)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

/// Test that returned function names are marked as used.
/// This fixes false positives for decorator wrappers and closures.
#[test]
fn test_return_statement_function_tracking() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("decorator.py");
    let mut file = File::create(&file_path).unwrap();

    // Decorator pattern where inner function is returned
    writeln!(
        file,
        r#"
def decorator(func):
    def wrapper(*args, **kwargs):
        print("Before")
        result = func(*args, **kwargs)
        print("After")
        return result
    return wrapper  # wrapper is returned, should be marked as used

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

    // wrapper should NOT be reported as unused (it's returned)
    assert!(
        !unused_func_names.contains(&"wrapper".to_owned()),
        "wrapper should not be flagged as unused since it is returned"
    );
}

/// Test that nested functions returned from outer functions are marked as used.
#[test]
fn test_return_nested_function() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("closure.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
def outer():
    def inner():
        return 42
    return inner  # inner is returned, should be used

result = outer()
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

    // inner should NOT be reported as unused
    assert!(
        !unused_func_names.contains(&"inner".to_owned()),
        "inner should not be flagged as unused since it is returned"
    );
}

/// Test that names in __all__ are marked as used.
#[test]
fn test_all_exports_marked_as_used() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("exports.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
__all__ = ["exported_func", "ExportedClass"]

def exported_func():
    pass

class ExportedClass:
    pass

def not_exported_func():
    pass

class NotExportedClass:
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

    let unused_class_names: Vec<String> = result
        .unused_classes
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    // Exported names should NOT be flagged
    assert!(
        !unused_func_names.contains(&"exported_func".to_owned()),
        "exported_func is in __all__ and should not be flagged"
    );
    assert!(
        !unused_class_names.contains(&"ExportedClass".to_owned()),
        "ExportedClass is in __all__ and should not be flagged"
    );

    // Non-exported names SHOULD be flagged
    assert!(
        unused_func_names.contains(&"not_exported_func".to_owned()),
        "not_exported_func is not in __all__ and should be flagged"
    );
    assert!(
        unused_class_names.contains(&"NotExportedClass".to_owned()),
        "NotExportedClass is not in __all__ and should be flagged"
    );
}

/// Test that imports inside TYPE_CHECKING blocks are not flagged as unused
/// ONLY IF they are actually used in type annotations.
/// Genuinely unused TYPE_CHECKING imports should still be flagged.
#[test]
fn test_type_checking_imports_not_flagged() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("type_hints.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from typing import List, Dict  # Used in string annotation below
    import json  # NOT used anywhere - should be flagged

def process(items: "List[Dict]") -> None:
    pass

process([])
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let unused_import_names: Vec<String> = result
        .unused_imports
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    // TYPE_CHECKING imports that ARE USED in annotations should NOT be flagged
    assert!(
        !unused_import_names.contains(&"List".to_owned()),
        "List is used in string annotation and should not be flagged"
    );
    assert!(
        !unused_import_names.contains(&"Dict".to_owned()),
        "Dict is used in string annotation and should not be flagged"
    );

    // TYPE_CHECKING imports that are NOT used should still be flagged
    assert!(
        unused_import_names.contains(&"json".to_owned()),
        "json is a genuinely unused TYPE_CHECKING import and SHOULD be flagged"
    );
}

/// Test TYPE_CHECKING with typing_extensions module.
/// OrderedDict is used in the return type annotation, so it should not be flagged.
#[test]
fn test_type_checking_typing_extensions() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("type_hints_ext.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
import typing_extensions

if typing_extensions.TYPE_CHECKING:
    from collections import OrderedDict  # Used in return annotation below

def get_data() -> "OrderedDict":
    return {{}}

get_data()
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let unused_import_names: Vec<String> = result
        .unused_imports
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    // TYPE_CHECKING import that IS USED in annotation should NOT be flagged
    assert!(
        !unused_import_names.contains(&"OrderedDict".to_owned()),
        "OrderedDict is used in return annotation and should not be flagged"
    );
}

/// Test that regular unused imports are still flagged correctly.
#[test]
fn test_regular_imports_still_flagged() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("imports.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
import os  # Unused
import sys  # Used

print(sys.version)
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let unused_import_names: Vec<String> = result
        .unused_imports
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    // os should be flagged as unused
    assert!(
        unused_import_names.contains(&"os".to_owned()),
        "os is unused and should be flagged"
    );

    // sys should NOT be flagged (it's used)
    assert!(
        !unused_import_names.contains(&"sys".to_owned()),
        "sys is used and should not be flagged"
    );
}
