//! Tests for CLI error handling and error message formatting.
//!
//! These tests verify that the CLI gracefully handles error conditions
//! and produces useful error messages.
#![allow(
    clippy::unwrap_used,
    clippy::uninlined_format_args,
    clippy::needless_raw_string_hashes
)]

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::config::Config;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

// =============================================================================
// Parse Error Handling Tests
// =============================================================================

#[test]
fn test_invalid_python_syntax_produces_parse_error() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("invalid.py");
    let mut file = File::create(&file_path).unwrap();
    // Invalid Python syntax - unclosed parenthesis
    writeln!(file, "def foo(:\n    pass").unwrap();

    let mut analyzer = CytoScnPy::new(
        60,    // confidence
        false, // secrets
        false, // danger
        false, // quality
        false, // include_tests
        vec![],
        vec![],
        false, // include_ipynb
        false, // ipynb_cells
        false, // taint
        Config::default(),
    );

    let result = analyzer.analyze(dir.path());

    // Should have parse errors but not crash
    assert!(
        !result.parse_errors.is_empty(),
        "Should report parse errors for invalid syntax"
    );
}

#[test]
fn test_analysis_continues_with_parse_errors() {
    let dir = tempdir().unwrap();

    // Create one valid file
    let valid_path = dir.path().join("valid.py");
    let mut valid_file = File::create(&valid_path).unwrap();
    writeln!(valid_file, "def unused_func():\n    pass").unwrap();

    // Create one invalid file
    let invalid_path = dir.path().join("invalid.py");
    let mut invalid_file = File::create(&invalid_path).unwrap();
    writeln!(invalid_file, "def broken(:\n    pass").unwrap();

    let mut analyzer = CytoScnPy::new(
        60,
        false,
        false,
        false,
        false,
        vec![],
        vec![],
        false,
        false,
        false,
        Config::default(),
    );

    let result = analyzer.analyze(dir.path());

    // Should have results from valid file
    assert!(
        !result.unused_functions.is_empty(),
        "Should still detect unused functions from valid files"
    );

    // Should also have parse errors
    assert!(
        !result.parse_errors.is_empty(),
        "Should report parse errors from invalid files"
    );
}

#[test]
fn test_parse_error_contains_file_path() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("broken_file.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "class Foo(\n    pass").unwrap();

    let mut analyzer = CytoScnPy::new(
        60,
        false,
        false,
        false,
        false,
        vec![],
        vec![],
        false,
        false,
        false,
        Config::default(),
    );

    let result = analyzer.analyze(dir.path());

    assert!(!result.parse_errors.is_empty());
    let error = &result.parse_errors[0];
    assert!(
        error.file.to_string_lossy().contains("broken_file.py"),
        "Parse error should include the file path"
    );
}

// =============================================================================
// Empty/Missing File Tests
// =============================================================================

#[test]
fn test_empty_python_file() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("empty.py");
    File::create(&file_path).unwrap(); // Create empty file

    let mut analyzer = CytoScnPy::new(
        60,
        false,
        false,
        false,
        false,
        vec![],
        vec![],
        false,
        false,
        false,
        Config::default(),
    );

    let result = analyzer.analyze(dir.path());

    // Should not crash, should have no findings
    assert!(result.unused_functions.is_empty());
    assert!(result.unused_imports.is_empty());
    assert!(result.parse_errors.is_empty());
}

#[test]
fn test_file_with_only_comments() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("comments.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "# This is a comment\n# Another comment").unwrap();

    let mut analyzer = CytoScnPy::new(
        60,
        false,
        false,
        false,
        false,
        vec![],
        vec![],
        false,
        false,
        false,
        Config::default(),
    );

    let result = analyzer.analyze(dir.path());

    // Should not crash or report errors
    assert!(result.parse_errors.is_empty());
}

// =============================================================================
// Edge Case Syntax Tests
// =============================================================================

#[test]
fn test_unicode_in_python_file() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("unicode.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "# -*- coding: utf-8 -*-").unwrap();
    writeln!(file, "message = \"Hello, 世界\"").unwrap();
    writeln!(file, "emoji = \"party\"").unwrap();

    let mut analyzer = CytoScnPy::new(
        60,
        false,
        false,
        false,
        false,
        vec![],
        vec![],
        false,
        false,
        false,
        Config::default(),
    );

    let result = analyzer.analyze(dir.path());

    // Should handle Unicode without errors
    assert!(result.parse_errors.is_empty());
}

#[test]
fn test_deeply_nested_syntax_error() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("nested_error.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(
        file,
        r#"
class Outer:
    class Inner:
        def method(self):
            if True:
                for i in range(10):
                    while True:
                        # Missing colon on next if
                        if x
                            pass
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::new(
        60,
        false,
        false,
        false,
        false,
        vec![],
        vec![],
        false,
        false,
        false,
        Config::default(),
    );

    let result = analyzer.analyze(dir.path());

    // Should detect parse error
    assert!(
        !result.parse_errors.is_empty(),
        "Should detect syntax error in nested code"
    );
}

// =============================================================================
// Analysis Summary Tests
// =============================================================================

#[test]
fn test_analysis_summary_counts_files() {
    let dir = tempdir().unwrap();

    // Create 5 Python files
    for i in 0..5 {
        let file_path = dir.path().join(format!("file{}.py", i));
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "x = {}", i).unwrap();
    }

    let mut analyzer = CytoScnPy::new(
        60,
        false,
        false,
        false,
        false,
        vec![],
        vec![],
        false,
        false,
        false,
        Config::default(),
    );

    let result = analyzer.analyze(dir.path());

    assert_eq!(
        result.analysis_summary.total_files, 5,
        "Should count all analyzed files"
    );
}

#[test]
fn test_analysis_paths_with_single_file() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("single.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "def unused(): pass").unwrap();

    let mut analyzer = CytoScnPy::new(
        60,
        false,
        false,
        false,
        false,
        vec![],
        vec![],
        false,
        false,
        false,
        Config::default(),
    );

    // Analyze single file instead of directory
    let result = analyzer.analyze_paths(&[file_path]);

    assert_eq!(result.analysis_summary.total_files, 1);
    assert!(!result.unused_functions.is_empty());
}
