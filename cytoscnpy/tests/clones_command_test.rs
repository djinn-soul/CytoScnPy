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
        include_tests: false,
        include_folders: vec![],
        verbose: false,
        with_cst: true,
        progress_bar: None,
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

#[path = "clones_command_test/cases_1.rs"]
mod cases_1;
