//! Comprehensive integration tests for the CLI entry point.
//!
//! These tests cover edge cases and flag combinations for the CLI.

#![allow(clippy::unwrap_used)]

use cytoscnpy::entry_point::run_with_args_to;
use tempfile::tempdir;

#[test]
fn test_cli_cc_all_flags_coverage() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def main(): pass").unwrap();

    let mut buffer = Vec::new();
    let args = vec![
        "cc".to_owned(),
        "--rank".to_owned(),
        "A".to_owned(),
        "--average".to_owned(),
        "--total-average".to_owned(),
        "--show-complexity".to_owned(),
        "--order".to_owned(),
        "score".to_owned(),
        "--no-assert".to_owned(),
        "--fail-threshold".to_owned(),
        "10".to_owned(),
        file_path.to_string_lossy().to_string(),
    ];
    let _ = run_with_args_to(args, &mut buffer).unwrap();
}

#[test]
fn test_cli_mi_all_flags_coverage() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def main(): pass").unwrap();

    let mut buffer = Vec::new();
    let args = vec![
        "mi".to_owned(),
        "--rank".to_owned(),
        "A".to_owned(),
        "--average".to_owned(),
        "--show".to_owned(),
        "--fail-threshold".to_owned(),
        "40.0".to_owned(),
        file_path.to_string_lossy().to_string(),
    ];
    let _ = run_with_args_to(args, &mut buffer).unwrap();
}

#[test]
fn test_cli_stats_fail_on_quality_coverage() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    // High complexity code to trigger quality issue
    std::fs::write(
        &file_path,
        "def f():\n if 1: \n  if 1: \n   if 1: \n    if 1: \n     if 1: \n      if 1: pass",
    )
    .unwrap();

    let mut buffer = Vec::new();
    let args = vec![
        "--fail-on-quality".to_owned(),
        "stats".to_owned(),
        "--all".to_owned(),
        file_path.to_string_lossy().to_string(),
    ];
    // This might return 1 if quality gate fails
    let _ = run_with_args_to(args, &mut buffer).unwrap();
}

#[test]
fn test_cli_invalid_output_path_coverage() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "pass").unwrap();

    let mut buffer = Vec::new();
    // Use an invalid output path (e.g. to a directory)
    let args = vec![
        "stats".to_owned(),
        "-o".to_owned(),
        dir.path().to_string_lossy().to_string(),
        file_path.to_string_lossy().to_string(),
    ];
    // should fail due to prepare_output_path
    let _ = run_with_args_to(args, &mut buffer);
}

#[test]
fn test_cli_deprecated_keys_coverage() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join(".cytoscnpy.toml");
    std::fs::write(&config_path, "[cytoscnpy]\ncomplexity = 10\n").unwrap();
    std::env::set_current_dir(&dir).unwrap();

    let mut buffer = Vec::new();
    let _ = run_with_args_to(vec!["stats".to_owned(), ".".to_owned()], &mut buffer).unwrap();
}

#[test]
fn test_cli_hal_functions_coverage() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def f(): x = 1").unwrap();

    let mut buffer = Vec::new();
    let args = vec![
        "hal".to_owned(),
        "--functions".to_owned(),
        file_path.to_string_lossy().to_string(),
    ];
    let _ = run_with_args_to(args, &mut buffer).unwrap();
}
