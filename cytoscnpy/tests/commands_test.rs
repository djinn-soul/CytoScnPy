//! Integration tests for CLI commands.
//!
//! This module tests the various command functions directly.

#![allow(clippy::unwrap_used)]

use cytoscnpy::commands::{
    run_cc, run_clones, run_hal, run_mi, run_raw, CcOptions, CloneOptions, MiOptions,
};
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_run_cc_coverage() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "def foo():\n    if True:\n        print('hi')").unwrap();
    let paths = vec![file.path().to_path_buf()];
    let mut output = Vec::new();
    let options = CcOptions {
        json: true,
        ..CcOptions::default()
    };
    run_cc(&paths, options, &mut output).unwrap();
}

#[test]
fn test_run_mi_coverage() {
    let mut file = tempfile::Builder::new().suffix(".py").tempfile().unwrap();
    writeln!(file, "def bar():\n    pass").unwrap();
    let paths = vec![file.path().to_path_buf()];
    let mut output = Vec::new();
    let options = MiOptions {
        json: true,
        ..MiOptions::default()
    };
    run_mi(&paths, options, &mut output).unwrap();
}

#[test]
fn test_run_raw_coverage() {
    let mut file = tempfile::Builder::new().suffix(".py").tempfile().unwrap();
    let code = "# comment\ndef baz():\n    pass";
    writeln!(file, "{code}").unwrap();
    let paths = vec![file.path().to_path_buf()];
    let mut output = Vec::new();
    run_raw(
        &paths,
        false,
        vec![],
        vec![],
        false,
        None,
        false,
        &mut output,
    )
    .unwrap();
    let output_str = String::from_utf8(output).unwrap();
    // The output should contain the filename, not necessarily the code content.
    assert!(
        output_str.contains(file.path().to_string_lossy().as_ref()),
        "Output should contain the filename. Got: {output_str}"
    );
}

#[test]
fn test_run_hal_coverage() {
    let mut file = tempfile::Builder::new().suffix(".py").tempfile().unwrap();
    writeln!(file, "x = 1 + 2").unwrap();
    let paths = vec![file.path().to_path_buf()];
    let mut output = Vec::new();
    run_hal(
        &paths,
        false,
        vec![],
        vec![],
        false,
        None,
        false,
        &mut output,
    )
    .unwrap();
}

#[test]
fn test_run_clones_coverage() {
    let mut file1 = tempfile::Builder::new().suffix(".py").tempfile().unwrap();
    let mut file2 = tempfile::Builder::new().suffix(".py").tempfile().unwrap();
    let code = "def some_func():\n    x = 1\n    y = 2\n    return x + y\n";
    writeln!(file1, "{code}").unwrap();
    writeln!(file2, "{code}").unwrap();

    let paths = vec![file1.path().to_path_buf(), file2.path().to_path_buf()];
    let mut output = Vec::new();
    let options = CloneOptions {
        similarity: 0.8,
        ..CloneOptions::default()
    };
    run_clones(&paths, &options, &mut output).unwrap();
}
