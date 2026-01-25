//! Integration tests for the application entry point.
//!
//! Tests the `run_with_args_to` function with various arguments.

#![allow(clippy::unwrap_used)]

use cytoscnpy::entry_point::run_with_args_to;
use tempfile::tempdir;

#[test]
fn test_cli_help_coverage() {
    let mut buffer = Vec::new();
    let result = run_with_args_to(vec!["--help".to_owned()], &mut buffer).unwrap();
    // clap returns error on help usually if not handled, but we handle it
    assert_eq!(result, 0);
}

#[test]
fn test_cli_invalid_path_coverage() {
    let mut buffer = Vec::new();
    let result = run_with_args_to(
        vec!["stats".to_owned(), "non_existent_file.py".to_owned()],
        &mut buffer,
    )
    .unwrap();
    assert!(result > 0);
}

#[test]
fn test_cli_init_coverage() {
    let dir = tempdir().unwrap();
    std::env::set_current_dir(&dir).unwrap();
    let mut buffer = Vec::new();
    let result = run_with_args_to(vec!["init".to_owned()], &mut buffer).unwrap();
    assert_eq!(result, 0);
    assert!(dir.path().join(".cytoscnpy.toml").exists());
}

#[test]
fn test_cli_stats_basic_coverage() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def main(): pass").unwrap();

    let mut buffer = Vec::new();
    let result = run_with_args_to(
        vec!["stats".to_owned(), file_path.to_string_lossy().to_string()],
        &mut buffer,
    )
    .unwrap();
    assert_eq!(result, 0);
}

#[test]
fn test_cli_mi_basic_coverage() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def main(): pass").unwrap();

    let mut buffer = Vec::new();
    let result = run_with_args_to(
        vec!["mi".to_owned(), file_path.to_string_lossy().to_string()],
        &mut buffer,
    )
    .unwrap();
    assert_eq!(result, 0);
}

#[test]
fn test_cli_cc_basic_coverage() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def main(): pass").unwrap();

    let mut buffer = Vec::new();
    let result = run_with_args_to(
        vec!["cc".to_owned(), file_path.to_string_lossy().to_string()],
        &mut buffer,
    )
    .unwrap();
    assert_eq!(result, 0);
}

#[test]
fn test_cli_hal_basic_coverage() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def main(): pass").unwrap();

    let mut buffer = Vec::new();
    let result = run_with_args_to(
        vec!["hal".to_owned(), file_path.to_string_lossy().to_string()],
        &mut buffer,
    )
    .unwrap();
    assert_eq!(result, 0);
}

#[test]
fn test_cli_raw_basic_coverage() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def main(): pass").unwrap();

    let mut buffer = Vec::new();
    let result = run_with_args_to(
        vec!["raw".to_owned(), file_path.to_string_lossy().to_string()],
        &mut buffer,
    )
    .unwrap();
    assert_eq!(result, 0);
}

#[test]
fn test_cli_stats_verbose_json_coverage() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def main(): pass").unwrap();

    let mut buffer = Vec::new();
    let result = run_with_args_to(
        vec![
            "--verbose".to_owned(),
            "stats".to_owned(),
            "--json".to_owned(),
            file_path.to_string_lossy().to_string(),
        ],
        &mut buffer,
    )
    .unwrap();
    assert_eq!(result, 0);
}

#[test]
fn test_cli_cc_xml_coverage() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def main(): pass").unwrap();

    let mut buffer = Vec::new();
    let result = run_with_args_to(
        vec![
            "cc".to_owned(),
            "--xml".to_owned(),
            file_path.to_string_lossy().to_string(),
        ],
        &mut buffer,
    )
    .unwrap();
    assert_eq!(result, 0);
}

#[test]
fn test_cli_mi_multi_coverage() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def main(): pass").unwrap();

    let mut buffer = Vec::new();
    let result = run_with_args_to(
        vec![
            "mi".to_owned(),
            "--multi".to_owned(),
            "false".to_owned(),
            file_path.to_string_lossy().to_string(),
        ],
        &mut buffer,
    )
    .unwrap();
    assert_eq!(result, 0);
}

#[test]
fn test_cli_raw_summary_coverage() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def main(): pass").unwrap();

    let mut buffer = Vec::new();
    let result = run_with_args_to(
        vec![
            "raw".to_owned(),
            "--summary".to_owned(),
            file_path.to_string_lossy().to_string(),
        ],
        &mut buffer,
    )
    .unwrap();
    assert_eq!(result, 0);
}

#[test]
fn test_cli_root_conflict_coverage() {
    let mut buffer = Vec::new();
    let result = run_with_args_to(
        vec![
            "--root".to_owned(),
            ".".to_owned(),
            "stats".to_owned(),
            "test.py".to_owned(),
        ],
        &mut buffer,
    )
    .unwrap();
    assert_eq!(result, 1); // validate_path_args should return 1
}

#[test]
fn test_cli_files_command_coverage() {
    let mut buffer = Vec::new();
    let result = run_with_args_to(vec!["files".to_owned(), ".".to_owned()], &mut buffer).unwrap();
    assert_eq!(result, 0);
}

#[test]
fn test_cli_version_coverage() {
    let mut buffer = Vec::new();
    let result = run_with_args_to(vec!["--version".to_owned()], &mut buffer).unwrap();
    assert_eq!(result, 0);
}

#[test]
fn test_cli_fix_flag_coverage() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def unused(): pass").unwrap();

    let mut buffer = Vec::new();
    let result = run_with_args_to(
        vec!["--fix".to_owned(), file_path.to_string_lossy().to_string()],
        &mut buffer,
    )
    .unwrap();
    assert_eq!(result, 0);
}
