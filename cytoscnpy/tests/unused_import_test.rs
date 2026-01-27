//! Tests for unused import detection.

use cytoscnpy::analyzer::CytoScnPy;
use std::path::PathBuf;

#[test]
fn test_unused_imports() {
    let cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(true);
    let code = r"
import os
import sys
import math

def func():
    pass
";
    let test_file_path = PathBuf::from("test.py");

    // Analyze the code
    let report = cytoscnpy.analyze_code(code, &test_file_path);

    // Verify that unused imports are detected
    // If the bug (self-reference) was present, these would be 0 and the test would fail.

    let found_os = report.unused_imports.iter().any(|i| i.simple_name == "os");
    assert!(found_os, "Should detect 'os' as unused import");

    let found_sys = report.unused_imports.iter().any(|i| i.simple_name == "sys");
    assert!(found_sys, "Should detect 'sys' as unused import");

    let found_math = report.unused_imports.iter().any(|i| i.simple_name == "math");
    assert!(found_math, "Should detect 'math' as unused import");
}
