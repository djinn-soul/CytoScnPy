//! Tests for parse error handling.
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::vec_init_then_push)]
#![allow(clippy::panic)]
#![allow(clippy::uninlined_format_args)]

use cytoscnpy::entry_point::run_with_args_to;
use serde_json::Value;
use std::str;

fn run_cytoscnpy(path: &str) -> Value {
    let mut args: Vec<String> = Vec::new();
    args.push(path.to_owned());
    args.push("--json".to_owned());

    let mut buffer = Vec::new();
    let _exit_code = run_with_args_to(args.clone(), &mut buffer)
        .unwrap_or_else(|e| panic!("Failed to run cytoscnpy with args {:?}: {}", args, e));

    let output_str = str::from_utf8(&buffer).expect("Invalid UTF-8 output");

    // We expect exit code 0 or 1 depending on errors, but for parse errors
    // we just want to verify the JSON output. The CLI might exit with 1 if issues found.
    // However, for parse errors they are part of the result.
    // Let's just parse the output.

    serde_json::from_str(output_str).expect("Failed to parse JSON output")
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
