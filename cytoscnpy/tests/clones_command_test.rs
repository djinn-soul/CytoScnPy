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

#[test]
fn test_clones_command_output_formatting() {
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

    let mut buffer = Vec::new();
    let options = CloneOptions {
        similarity: 0.8,
        json: false, // Text output
        fix: false,
        dry_run: true,
        exclude: vec![],
        verbose: true,
        with_cst: true,
    };

    let _ = run_clones(std::slice::from_ref(&path_str), &options, &mut buffer)
        .expect("Clones command failed");

    let output = String::from_utf8(buffer).expect("Invalid UTF-8 output");

    // Check for table headers and content
    assert!(output.contains("Clone Detection Results"));
    assert!(output.contains("Similarity"));
    assert!(output.contains("Suggestion"));
    // Check if the clone function name appears
    assert!(output.contains("exact_copy"));
}

#[test]
fn test_clones_command_fix_execution() {
    // This test verifies that the 'fix' option actually modifies the file.
    // Since we can't easily run full refactoring in this unit test environment without
    // potentially more setup, we will basic removal logic which is the default for duplication.

    // Using Type 1 clones which default to "Remove duplicate"
    let source1 = "
def exact_copy(x):
    return x * 2
";
    let source2 = "
def exact_copy(x):
    return x * 2
";
    let temp_dir = create_temp_project(&[("file1.py", source1), ("file2.py", source2)]);
    let path1 = temp_dir.path().join("file1.py");
    let path2 = temp_dir.path().join("file2.py");
    let path_str = temp_dir.path().to_path_buf();

    let mut buffer = Vec::new();
    let options = CloneOptions {
        similarity: 0.9,
        json: false,
        fix: true,
        dry_run: false, // Actually apply fixes
        exclude: vec![],
        verbose: false,
        with_cst: false, // Byte-based for simplicity
    };

    let _ = run_clones(std::slice::from_ref(&path_str), &options, &mut buffer)
        .expect("Clones command failed");

    // After fix, one of the files should be modified (content removed)
    // or both if it decides to remove both (depends on logic, but usually keeps canonical)
    let content1 = fs::read_to_string(&path1).unwrap();
    let content2 = fs::read_to_string(&path2).unwrap();

    // clone removal replaces with empty string or similar in byte-rewriter?
    // ByteRangeRewriter::delete removes the range.
    // One file should be smaller than original (original len approx 35 bytes)
    assert!(
        content1.len() < 10 || content2.len() < 10,
        "One file should have been reduced (duplicate removed)"
    );
}

#[test]
fn test_clones_command_dry_run() {
    let source1 = "
def exact_copy(x):
    return x * 2
";
    let source2 = "
def exact_copy(x):
    return x * 2
";
    let temp_dir = create_temp_project(&[("file1.py", source1), ("file2.py", source2)]);
    let path1 = temp_dir.path().join("file1.py");
    let path2 = temp_dir.path().join("file2.py");
    let path_str = temp_dir.path().to_path_buf();

    let mut buffer = Vec::new();
    let options = CloneOptions {
        similarity: 0.9,
        json: false,
        fix: true,
        dry_run: true, // Should NOT modify files
        exclude: vec![],
        verbose: true,
        with_cst: false,
    };

    let _ = run_clones(std::slice::from_ref(&path_str), &options, &mut buffer)
        .expect("Clones command failed");

    let output = String::from_utf8(buffer).expect("Invalid UTF-8 output");

    // Verify dry run message
    assert!(output.contains("[DRY-RUN]"));
    assert!(output.contains("Would remove"));

    // Verify files are UNCHANGED
    let content1 = fs::read_to_string(&path1).unwrap();
    let content2 = fs::read_to_string(&path2).unwrap();

    assert!(content1.contains("def exact_copy"));
    assert!(content2.contains("def exact_copy"));
    assert!(content1.len() > 20);
    assert!(content2.len() > 20);
}
