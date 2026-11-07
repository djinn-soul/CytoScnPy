//! Integration tests for cytoscnpy
//! These tests run the binary on sample code and verify output matches expectations
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::panic)]

use cytoscnpy::entry_point::run_with_args_to;
use serde_json::Value;
use std::str;

/// Helper function to run cytoscnpy and parse JSON output
fn run_cytoscnpy(path: &str, flags: &[&str]) -> Value {
    let mut args: Vec<String> = Vec::new();
    // Default to the same behavior as CLI: just passed args
    // integration tests pass path first
    args.push(path.to_owned());
    for flag in flags {
        args.push((*flag).to_owned());
    }
    args.push("--json".to_owned());

    let mut buffer = Vec::new();
    let exit_code = run_with_args_to(args.clone(), &mut buffer)
        .unwrap_or_else(|e| panic!("Failed to run cytoscnpy with args {:?}: {}", args, e));

    let output_str = str::from_utf8(&buffer).expect("Invalid UTF-8 output");

    assert_eq!(exit_code, 0, "Command failed with exit code {exit_code}");

    serde_json::from_str(output_str).expect("Failed to parse JSON output")
}

/// Helper to count items in a JSON array field
fn count_items(result: &Value, field: &str) -> usize {
    result[field].as_array().map_or(0, std::vec::Vec::len)
}

/// Helper to check if an item with a specific name exists in results
fn has_item_named(result: &Value, field: &str, name: &str) -> bool {
    result[field].as_array().is_some_and(|arr| {
        arr.iter().any(|item| {
            item["simple_name"].as_str() == Some(name) || item["name"].as_str() == Some(name)
        })
    })
}

use std::fs;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-integration-tmp");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("integration_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[path = "integration_test/cases_1.rs"]
mod cases_1;
