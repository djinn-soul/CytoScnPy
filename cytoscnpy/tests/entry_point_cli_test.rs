//! Tests for entry_point.rs CLI argument handling and run_with_args function.
#![allow(clippy::unwrap_used)]

use cytoscnpy::entry_point::run_with_args;
use std::fs;
use tempfile::tempdir;

/// Test that --version flag works correctly.
#[test]
fn test_version_flag() {
    let result = run_with_args(vec!["--version".to_owned()]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test that --help flag works correctly.
#[test]
fn test_help_flag() {
    let result = run_with_args(vec!["--help".to_owned()]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test analyzing a single Python file.
#[test]
fn test_analyze_single_file() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_file.py");
    fs::write(&file_path, "def unused_func():\n    pass\n").unwrap();

    let result = run_with_args(vec![
        "--json".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
}

/// Test analyzing with --secrets flag.
#[test]
fn test_secrets_flag() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("secrets_test.py");
    fs::write(&file_path, "API_KEY = 'sk-1234567890abcdef'\n").unwrap();

    let result = run_with_args(vec![
        "--json".to_owned(),
        "--secrets".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
}

/// Test analyzing with --danger flag.
#[test]
fn test_danger_flag() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("danger_test.py");
    fs::write(&file_path, "import os\nos.system('ls')\n").unwrap();

    let result = run_with_args(vec![
        "--json".to_owned(),
        "--danger".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
}

/// Test analyzing with --quality flag.
#[test]
fn test_quality_flag() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("quality_test.py");
    fs::write(
        &file_path,
        "def complex_func():\n    if True:\n        if True:\n            pass\n",
    )
    .unwrap();

    let result = run_with_args(vec![
        "--json".to_owned(),
        "--quality".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
}

/// Test error handling for non-existent path.
#[test]
fn test_nonexistent_path() {
    let result = run_with_args(vec![
        "--json".to_owned(),
        "/nonexistent/path/to/file.py".to_owned(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1); // Should return error code 1
}

/// Test the `raw` subcommand.
#[test]
fn test_raw_subcommand() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("raw_test.py");
    fs::write(&file_path, "x = 1\ny = 2\n").unwrap();

    let result = run_with_args(vec![
        "raw".to_owned(),
        "--json".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test the `cc` (cyclomatic complexity) subcommand.
#[test]
fn test_cc_subcommand() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("cc_test.py");
    fs::write(&file_path, "def foo():\n    if True:\n        pass\n").unwrap();

    let result = run_with_args(vec![
        "cc".to_owned(),
        "--json".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test the `hal` (Halstead metrics) subcommand.
#[test]
fn test_hal_subcommand() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("hal_test.py");
    fs::write(&file_path, "x = 1 + 2 * 3\n").unwrap();

    let result = run_with_args(vec![
        "hal".to_owned(),
        "--json".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test the `mi` (Maintainability Index) subcommand.
#[test]
fn test_mi_subcommand() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("mi_test.py");
    fs::write(&file_path, "def foo():\n    pass\n").unwrap();

    let result = run_with_args(vec![
        "mi".to_owned(),
        "--json".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test the `stats` subcommand.
#[test]
fn test_stats_subcommand() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("stats_test.py");
    fs::write(&file_path, "def foo():\n    pass\n").unwrap();

    let result = run_with_args(vec![
        "stats".to_owned(),
        "--json".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test the `files` subcommand.
#[test]
fn test_files_subcommand() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("files_test.py");
    fs::write(&file_path, "x = 1\n").unwrap();

    let result = run_with_args(vec![
        "files".to_owned(),
        "--json".to_owned(),
        dir.path().to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test --verbose flag.
#[test]
fn test_verbose_flag() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("verbose_test.py");
    fs::write(&file_path, "def foo():\n    pass\n").unwrap();

    let result = run_with_args(vec![
        "--verbose".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
}

/// Test --confidence flag.
#[test]
fn test_confidence_flag() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("confidence_test.py");
    fs::write(&file_path, "def maybe_used():\n    pass\n").unwrap();

    let result = run_with_args(vec![
        "--json".to_owned(),
        "--confidence".to_owned(),
        "80".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
}

/// Test --exclude-folders flag.
#[test]
fn test_exclude_folders_flag() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("exclude_test.py");
    fs::write(&file_path, "def foo():\n    pass\n").unwrap();

    let result = run_with_args(vec![
        "--json".to_owned(),
        "--exclude-folders".to_owned(),
        "tests".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
}
