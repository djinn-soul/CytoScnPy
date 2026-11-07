//! Tests for Azure Functions framework detection and taint analysis.
#![allow(clippy::expect_used)]

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::constants::get_framework_file_re;
use std::fs;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().expect("Failed to get current dir");
    target_dir.push("target");
    target_dir.push("test-azure-tmp");
    fs::create_dir_all(&target_dir).expect("Failed to create test target dir");
    tempfile::Builder::new()
        .prefix("azure_test_")
        .tempdir_in(target_dir)
        .expect("Failed to create tempdir")
}

/// Helper function to analyze a Python file and return the results
fn analyze_code(code: &str) -> cytoscnpy::analyzer::AnalysisResult {
    let dir = project_tempdir();
    let file_path = dir.path().join("test.py");
    fs::write(&file_path, code).expect("Failed to write test file");

    let mut analyzer = CytoScnPy::default()
        .with_confidence(60)
        .with_tests(false)
        .with_quality(false)
        .with_danger(true); // Enable danger to activate taint analysis

    analyzer.analyze(dir.path())
}

#[path = "azure_functions_test/cases_1.rs"]
mod cases_1;
#[path = "azure_functions_test/cases_2.rs"]
mod cases_2;
#[path = "azure_functions_test/cases_3.rs"]
mod cases_3;
