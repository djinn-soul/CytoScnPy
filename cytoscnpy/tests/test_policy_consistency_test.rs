//! Regression tests for consistent test-file inclusion across CLI commands.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use cytoscnpy::entry_point::run_with_args_to;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn create_project() -> TempDir {
    let project = tempfile::tempdir().unwrap();
    let tests = project.path().join("tests");
    fs::create_dir_all(&tests).unwrap();
    fs::write(
        project.path().join("main.py"),
        "def production(value):\n    return value + 1\n",
    )
    .unwrap();
    fs::write(
        tests.join("test_main.py"),
        "def bad(values=[]):\n    if True == flag:\n        return values\n",
    )
    .unwrap();
    project
}

fn run_json(args: Vec<String>) -> Value {
    let mut output = Vec::new();
    let exit_code = run_with_args_to(args, &mut output).expect("CLI should run");
    assert_eq!(exit_code, 0);
    serde_json::from_slice(&output).expect("CLI should emit JSON")
}

fn command_json(project: &TempDir, command: &str, include_tests: bool) -> Value {
    let mut args = Vec::new();
    if include_tests {
        args.push("--include-tests".to_owned());
    }
    args.extend([
        command.to_owned(),
        "--json".to_owned(),
        project.path().to_string_lossy().to_string(),
    ]);
    run_json(args)
}

fn json_mentions_test_file(value: &Value) -> bool {
    value.to_string().contains("test_main.py")
}

#[test]
fn metric_and_files_commands_share_test_policy() {
    let project = create_project();

    for command in ["raw", "cc", "hal", "mi", "files"] {
        let without_tests = command_json(&project, command, false);
        assert!(
            !json_mentions_test_file(&without_tests),
            "{command} should exclude tests by default"
        );

        let with_tests = command_json(&project, command, true);
        assert!(
            json_mentions_test_file(&with_tests),
            "{command} should include tests when requested"
        );
    }
}

#[test]
fn main_analysis_excludes_tests_unless_requested() {
    let project = create_project();
    let root = project.path().to_string_lossy().to_string();

    let without_tests = run_json(vec![
        "--json".to_owned(),
        "--quality".to_owned(),
        root.clone(),
    ]);
    assert!(!json_mentions_test_file(&without_tests));

    let with_tests = run_json(vec![
        "--json".to_owned(),
        "--quality".to_owned(),
        "--include-tests".to_owned(),
        root,
    ]);
    assert!(json_mentions_test_file(&with_tests));
}

#[test]
fn stats_totals_and_findings_share_test_policy() {
    let project = create_project();
    let root = project.path().to_string_lossy().to_string();

    let without_tests = run_json(vec![
        "stats".to_owned(),
        "--quality".to_owned(),
        "--json".to_owned(),
        root.clone(),
    ]);
    assert_eq!(without_tests["total_files"], 1);
    assert!(!json_mentions_test_file(&without_tests));

    let with_tests = run_json(vec![
        "--include-tests".to_owned(),
        "stats".to_owned(),
        "--quality".to_owned(),
        "--json".to_owned(),
        root,
    ]);
    assert_eq!(with_tests["total_files"], 2);
    assert!(json_mentions_test_file(&with_tests));
}
