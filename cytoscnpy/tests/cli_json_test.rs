//! Tests for CLI JSON output format validation.
//!
//! These tests verify that JSON output from all subcommands is correctly structured
//! and can be deserialized by external tools.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::uninlined_format_args)]

use cytoscnpy::commands::{run_cc, run_hal, run_mi, run_raw};
use serde_json::Value;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

// =============================================================================
// JSON Output Structure Tests
// =============================================================================

#[test]
fn test_raw_json_output_structure() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "x = 1\n# comment\ny = 2").unwrap();

    let mut buffer = Vec::new();
    run_raw(
        dir.path(),
        true, // json=true
        vec![],
        Vec::new(),
        false,
        None,
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    let json: Value = serde_json::from_str(&output).expect("Output should be valid JSON");

    // Should be an array of results
    assert!(json.is_array(), "JSON output should be an array");

    let results = json.as_array().unwrap();
    assert!(!results.is_empty(), "Should have at least one result");

    // Check expected fields
    let first = &results[0];
    assert!(first.get("file").is_some(), "Should have 'file' field");
    assert!(first.get("loc").is_some(), "Should have 'loc' field");
    assert!(first.get("sloc").is_some(), "Should have 'sloc' field");
    assert!(first.get("lloc").is_some(), "Should have 'lloc' field");
    assert!(
        first.get("comments").is_some(),
        "Should have 'comments' field"
    );
    assert!(first.get("multi").is_some(), "Should have 'multi' field");
    assert!(first.get("blank").is_some(), "Should have 'blank' field");
}

#[test]
fn test_cc_json_output_structure() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "def foo():\n    if True:\n        pass").unwrap();

    let mut buffer = Vec::new();
    run_cc(
        dir.path(),
        cytoscnpy::commands::CcOptions {
            json: true,
            exclude: vec![],
            ignore: Vec::new(),
            output_file: None,
            ..Default::default()
        },
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    let json: Value = serde_json::from_str(&output).expect("Output should be valid JSON");

    assert!(json.is_array(), "JSON output should be an array");

    let results = json.as_array().unwrap();
    assert!(!results.is_empty(), "Should have at least one result");

    let first = &results[0];
    assert!(first.get("file").is_some(), "Should have 'file' field");
    assert!(first.get("name").is_some(), "Should have 'name' field");
    assert!(first.get("type_").is_some(), "Should have 'type_' field");
    assert!(first.get("line").is_some(), "Should have 'line' field");
    assert!(
        first.get("complexity").is_some(),
        "Should have 'complexity' field"
    );
    assert!(first.get("rank").is_some(), "Should have 'rank' field");
}

#[test]
fn test_mi_json_output_structure() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "x = 1\ny = 2").unwrap();

    let mut buffer = Vec::new();
    run_mi(
        dir.path(),
        cytoscnpy::commands::MiOptions {
            json: true,
            exclude: vec![],
            ignore: Vec::new(),
            output_file: None,
            ..Default::default()
        },
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    let json: Value = serde_json::from_str(&output).expect("Output should be valid JSON");

    assert!(json.is_array(), "JSON output should be an array");

    let results = json.as_array().unwrap();
    assert!(!results.is_empty(), "Should have at least one result");

    let first = &results[0];
    assert!(first.get("file").is_some(), "Should have 'file' field");
    assert!(first.get("mi").is_some(), "Should have 'mi' field");
    assert!(first.get("rank").is_some(), "Should have 'rank' field");
}

#[test]
fn test_hal_json_output_structure() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "x = 1 + 2\ny = x * 3").unwrap();

    let mut buffer = Vec::new();
    run_hal(
        dir.path(),
        true, // json=true
        vec![],
        Vec::new(),
        false,
        None,
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    let json: Value = serde_json::from_str(&output).expect("Output should be valid JSON");

    assert!(json.is_array(), "JSON output should be an array");

    let results = json.as_array().unwrap();
    assert!(!results.is_empty(), "Should have at least one result");

    let first = &results[0];
    assert!(first.get("file").is_some(), "Should have 'file' field");
    assert!(first.get("h1").is_some(), "Should have 'h1' field");
    assert!(first.get("h2").is_some(), "Should have 'h2' field");
    assert!(first.get("n1").is_some(), "Should have 'n1' field");
    assert!(first.get("n2").is_some(), "Should have 'n2' field");
    assert!(
        first.get("vocabulary").is_some(),
        "Should have 'vocabulary' field"
    );
    assert!(first.get("volume").is_some(), "Should have 'volume' field");
    assert!(
        first.get("difficulty").is_some(),
        "Should have 'difficulty' field"
    );
    assert!(first.get("effort").is_some(), "Should have 'effort' field");
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[test]
fn test_json_empty_directory() {
    let dir = tempdir().unwrap();
    // Create multiple Python files
    for i in 0..3 {
        let file_path = dir.path().join(format!("test{}.py", i));
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "x{} = {}", i, i).unwrap();
    }

    let mut buffer = Vec::new();
    run_raw(
        dir.path(),
        true,
        vec![],
        Vec::new(),
        false,
        None,
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    let json: Value = serde_json::from_str(&output).expect("Output should be valid JSON");

    let results = json.as_array().unwrap();
    // At least 3 Python files should be in the results
    assert!(
        results.len() >= 3,
        "Should have at least 3 results for 3 files, got {}",
        results.len()
    );
}

#[test]
fn test_json_numeric_values_are_numbers() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "x = 1\ny = 2\nz = 3").unwrap();

    let mut buffer = Vec::new();
    run_raw(
        dir.path(),
        true,
        vec![],
        Vec::new(),
        false,
        None,
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    let json: Value = serde_json::from_str(&output).unwrap();

    let first = &json.as_array().unwrap()[0];

    // Numeric fields should be numbers, not strings
    assert!(first["loc"].is_number(), "'loc' should be a number");
    assert!(first["sloc"].is_number(), "'sloc' should be a number");
    assert!(first["blank"].is_number(), "'blank' should be a number");
}

#[test]
fn test_cc_json_complexity_value_types() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "def foo(): pass").unwrap();

    let mut buffer = Vec::new();
    run_cc(
        dir.path(),
        cytoscnpy::commands::CcOptions {
            json: true,
            exclude: vec![],
            ignore: Vec::new(),
            output_file: None,
            ..Default::default()
        },
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    let json: Value = serde_json::from_str(&output).unwrap();

    let first = &json.as_array().unwrap()[0];

    assert!(
        first["complexity"].is_number(),
        "'complexity' should be a number"
    );
    assert!(first["line"].is_number(), "'line' should be a number");
    assert!(first["rank"].is_string(), "'rank' should be a string");
}
