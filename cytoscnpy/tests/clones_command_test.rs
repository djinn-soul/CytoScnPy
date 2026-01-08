//! Tests for the clone detection command.
//!
//! These tests verify the `clones` subcommand functionality including
//! similarity thresholds, exclude patterns, and JSON output format.

// Test-specific lint suppressions - these patterns are idiomatic in tests
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use cytoscnpy::commands::{run_clones, CloneOptions};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to run cytoscnpy clones command in-process
fn run_clones_api(paths: &[PathBuf], args: &CloneArgs) -> serde_json::Value {
    let mut buffer = Vec::new();

    let options = CloneOptions {
        similarity: args.similarity,
        json: true,
        fix: false,
        dry_run: true,
        exclude: args.exclude.clone(),
        verbose: false,
        with_cst: true,
    };

    let _ = run_clones(paths, &options, &mut buffer).expect("Clones command failed");

    let output = String::from_utf8(buffer).expect("Invalid UTF-8 output");
    serde_json::from_str(&output).expect("Failed to parse JSON output")
}

#[derive(Default)]
struct CloneArgs {
    similarity: f64,
    exclude: Vec<String>,
}

fn create_temp_project(files: &[(&str, &str)]) -> TempDir {
    let dir = tempfile::Builder::new()
        .prefix("clones_test_")
        .tempdir()
        .unwrap();

    for (name, content) in files {
        if name.contains('/') {
            let path = dir.path().join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, content).unwrap();
        } else {
            let path = dir.path().join(name);
            fs::write(path, content).unwrap();
        }
    }

    dir
}

#[test]
fn test_clones_command_basic() {
    let source1 = "
def exact_copy(x):
    return x * 2
";
    let source2 = "
def exact_copy(x):
    return x * 2
";

    let temp_dir = create_temp_project(&[("file1.py", source1), ("file2.py", source2)]);

    let path_str = temp_dir.path().to_path_buf();
    let json = run_clones_api(
        &[path_str],
        &CloneArgs {
            similarity: 0.8,
            ..Default::default()
        },
    );

    // In-process, we get [Finding, Finding, ...].
    // run_clones directly returns the list of clone findings if json=true

    let findings = json.as_array().expect("Output should be a JSON array");

    assert!(findings.len() >= 2);

    let finding = &findings[0];
    assert_eq!(finding["clone_type"], "Type1");
    assert!(finding["similarity"].as_f64().unwrap() > 0.99);
}

#[test]
fn test_clones_command_similarity_threshold() {
    let source1 = "
def func_a(x):
    a = 1
    b = 2
    return x + a + b
";
    let source2 = "
def func_b(y):
    # structurally different enough
    c = 1
    d = 3
    print('extra') 
    return y + c + d
";

    let temp_dir = create_temp_project(&[("file1.py", source1), ("file2.py", source2)]);
    let path_str = temp_dir.path().to_path_buf();

    let json_high = run_clones_api(
        std::slice::from_ref(&path_str),
        &CloneArgs {
            similarity: 1.0,
            ..Default::default()
        },
    );
    assert_eq!(json_high.as_array().unwrap().len(), 0);

    let _json_low = run_clones_api(
        &[path_str],
        &CloneArgs {
            similarity: 0.5,
            ..Default::default()
        },
    );
}

#[test]
fn test_clones_command_exclude() {
    let source = "
def duplicate(x):
    return x
";
    let temp_dir = create_temp_project(&[("included.py", source), ("subdir/excluded.py", source)]);
    let path_str = temp_dir.path().to_path_buf();

    let json = run_clones_api(
        &[path_str],
        &CloneArgs {
            similarity: 0.8,
            exclude: vec!["subdir".to_owned()],
        },
    );

    assert_eq!(json.as_array().unwrap().len(), 0);
}
