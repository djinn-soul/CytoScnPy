//! Tests for parse error handling.
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use serde_json::Value;
use std::process::Command;
use std::str;

fn run_cytoscnpy(path: &str) -> Value {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--package", "cytoscnpy-cli", "--"])
        .arg(path)
        .arg("--json")
        .output()
        .expect("Failed to execute cytoscnpy binary");

    assert!(
        output.status.success(),
        "Command failed: {}",
        str::from_utf8(&output.stderr).unwrap_or("")
    );

    let stdout = str::from_utf8(&output.stdout).expect("Invalid UTF-8 output");
    serde_json::from_str(stdout).expect("Failed to parse JSON output")
}

#[test]
fn test_parse_errors() {
    let result = run_cytoscnpy("tests/data/bad_syntax");

    let parse_errors = result["parse_errors"]
        .as_array()
        .expect("parse_errors should be an array");

    // We created 5 files, all should fail parsing
    assert_eq!(parse_errors.len(), 5, "Should report 5 parse errors");

    let error_files: Vec<&str> = parse_errors
        .iter()
        .map(|e| e["file"].as_str().unwrap())
        .collect();

    assert!(error_files.iter().any(|f| f.contains("missing_paren.py")));
    assert!(error_files.iter().any(|f| f.contains("bad_indent.py")));
    assert!(error_files.iter().any(|f| f.contains("print_statement.py")));
    assert!(error_files.iter().any(|f| f.contains("incomplete.py")));
    assert!(error_files.iter().any(|f| f.contains("invalid_token.py")));
}
