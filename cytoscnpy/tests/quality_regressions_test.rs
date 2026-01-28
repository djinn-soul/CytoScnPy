//! Integration tests for quality regressions and gatekeepers.

#![allow(clippy::expect_used)]

use cytoscnpy::entry_point::run_with_args_to;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

fn run_json(args: Vec<String>) -> (i32, Value) {
    let mut output = Vec::new();
    let exit_code = run_with_args_to(args, &mut output).expect("CLI run failed");
    let stdout = String::from_utf8(output).expect("Invalid UTF-8 output");
    let json: Value = serde_json::from_str(&stdout).expect("Failed to parse JSON output");
    (exit_code, json)
}

#[test]
fn test_max_complexity_cli_override_triggers_gate() {
    let dir = project_tempdir();
    let file_path = dir.path().join("complex.py");
    fs::write(
        &file_path,
        r"
def complex(a, b, c):
    if a:
        pass
    elif b:
        pass
    elif c:
        pass
    else:
        pass
",
    )
    .expect("Failed to write test file");

    let (exit_code, json) = run_json(vec![
        "--json".to_owned(),
        "--quality".to_owned(),
        "--max-complexity".to_owned(),
        "3".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);

    assert_eq!(
        exit_code, 1,
        "Expected complexity gate to fail with exit code 1"
    );

    let has_complexity = json["quality"]
        .as_array()
        .expect("quality should be an array")
        .iter()
        .any(|f| f["rule_id"] == cytoscnpy::rules::ids::RULE_ID_COMPLEXITY);
    assert!(has_complexity, "Expected a CSP-Q301 complexity finding");
}

#[test]
fn test_quality_rule_id_suppression_works() {
    let dir = project_tempdir();
    let file_path = dir.path().join("suppressed.py");
    fs::write(
        &file_path,
        "def bad(x=[]):  # noqa: CSP-L001\n    return x\n",
    )
    .expect("Failed to write test file");

    let (exit_code, json) = run_json(vec![
        "--json".to_owned(),
        "--quality".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);

    assert_eq!(
        exit_code, 0,
        "Suppressed quality issue should not fail the run"
    );

    let has_mutable_default = json["quality"]
        .as_array()
        .expect("quality should be an array")
        .iter()
        .any(|f| f["rule_id"] == cytoscnpy::rules::ids::RULE_ID_MUTABLE_DEFAULT);
    assert!(
        !has_mutable_default,
        "Expected CSP-L001 mutable default to be suppressed"
    );
}

#[test]
fn test_dangerous_comparison_left_hand_literal_detected() {
    let dir = project_tempdir();
    let file_path = dir.path().join("dangerous_compare.py");
    fs::write(&file_path, "if True == flag:\n    pass\n").expect("Failed to write test file");

    let (_exit_code, json) = run_json(vec![
        "--json".to_owned(),
        "--quality".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);

    let has_dangerous_comparison = json["quality"]
        .as_array()
        .expect("quality should be an array")
        .iter()
        .any(|f| f["rule_id"] == cytoscnpy::rules::ids::RULE_ID_DANGEROUS_COMPARISON);
    assert!(
        has_dangerous_comparison,
        "Expected CSP-L003 dangerous comparison finding"
    );
}
