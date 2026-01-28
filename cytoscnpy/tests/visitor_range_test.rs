//! Tests for visitor range precision and fix suggestions.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::path::Path;

#[test]
fn test_precise_visitor_ranges() {
    let code = r"
import os
import sys

def foo(x):
    return x * 2

class Bar:
    def baz(self):
        pass
";

    let analyzer = CytoScnPy::default();
    let result = analyzer.analyze_code(code, Path::new("test.py"));

    // 1. Verify Import Range
    let import_os = result
        .unused_imports
        .iter()
        .find(|d| d.simple_name == "os")
        .expect("os import not found");

    let os_start = code.find("os").unwrap();
    let os_end = os_start + "os".len();

    assert_eq!(
        import_os.start_byte, os_start,
        "Import 'os' start byte mismatch"
    );
    assert_eq!(import_os.end_byte, os_end, "Import 'os' end byte mismatch");

    // 2. Verify FunctionDef Range & FIX
    let func_foo = result
        .unused_functions
        .iter()
        .find(|d| d.simple_name == "foo")
        .expect("foo function not found");

    let foo_start = code.find("foo(x)").unwrap();
    assert_eq!(
        func_foo.start_byte, foo_start,
        "Function 'foo' start byte mismatch"
    );

    let body_start = code.find("return x * 2").unwrap();
    assert!(
        func_foo.end_byte >= body_start + "return x * 2".len(),
        "Function 'foo' end byte too short"
    );

    // TDD: Verify Fix exists and covers the full range
    assert!(
        func_foo.fix.is_some(),
        "Function 'foo' should have a fix suggestion"
    );
    let fix = func_foo.fix.as_ref().unwrap();
    assert_eq!(
        fix.start_byte,
        code.find("def foo(x):").unwrap(),
        "Fix should start at 'def', coverage: full definition"
    );
    assert_eq!(fix.end_byte, func_foo.end_byte, "Fix end byte mismatch");
    assert_eq!(
        fix.replacement, "",
        "Fix should be a deletion (empty replacement)"
    );

    // 3. Verify ClassDef Range & FIX
    let class_bar = result
        .unused_classes
        .iter()
        .find(|d| d.simple_name == "Bar")
        .expect("Bar class not found");

    let bar_start = code.find("Bar:").unwrap();
    assert_eq!(
        class_bar.start_byte, bar_start,
        "Class 'Bar' start byte mismatch"
    );

    let method_pass = code.find("pass").unwrap();
    assert!(
        class_bar.end_byte >= method_pass + 4,
        "Class 'Bar' end byte too short"
    );

    // Verify Class Fix
    assert!(
        class_bar.fix.is_some(),
        "Class 'Bar' should have a fix suggestion"
    );
    let fix_cls = class_bar.fix.as_ref().unwrap();
    assert_eq!(
        fix_cls.start_byte,
        code.find("class Bar:").unwrap(),
        "Class fix start byte mismatch"
    );
    assert_eq!(
        fix_cls.end_byte, class_bar.end_byte,
        "Class fix end byte mismatch"
    );

    // 4. Verify MethodDef Range
    let method_baz = result
        .unused_methods
        .iter()
        .find(|d| d.simple_name == "baz")
        .expect("baz method not found");

    let method_baz_start = code.find("baz(self)").unwrap();
    assert_eq!(
        method_baz.start_byte, method_baz_start,
        "Method 'baz' start byte mismatch"
    );
    assert!(
        method_baz.end_byte >= method_pass + 4,
        "Method 'baz' end byte too short"
    );

    // Method Fix
    assert!(
        method_baz.fix.is_some(),
        "Method 'baz' should have a fix suggestion"
    );
    let fix_meth = method_baz.fix.as_ref().unwrap();
    assert_eq!(
        fix_meth.start_byte,
        code.find("def baz(self):").unwrap(),
        "Method fix start byte mismatch"
    );
    assert_eq!(
        fix_meth.end_byte, method_baz.end_byte,
        "Method fix end byte mismatch"
    );

    println!("Ranges and Fixes verified successfully!");
}
