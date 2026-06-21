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

fn has_rule(json: &Value, category: &str, rule_id: &str) -> bool {
    json[category]
        .as_array()
        .expect("category should be an array")
        .iter()
        .any(|f| f["rule_id"] == rule_id)
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

#[test]
fn test_path_aware_danger_rules_suppress_test_support_files() {
    let dir = project_tempdir();
    let production_path = dir.path().join("app.py");
    let tests_dir = dir.path().join("tests");
    fs::create_dir(&tests_dir).expect("Failed to create tests dir");
    let test_path = tests_dir.join("test_app.py");
    let nox_path = dir.path().join("noxfile.py");

    let source = r#"
import hashlib
import os


def load_config(value, password, path):
    eval(value)
    exec(value)
    assert value
    if password == "password":
        pass
    if os.path.exists(path):
        open(path).read()
    hashlib.md5(b"demo").hexdigest()
"#;
    fs::write(&production_path, source).expect("Failed to write production file");
    fs::write(&test_path, source).expect("Failed to write test file");
    fs::write(&nox_path, source).expect("Failed to write noxfile");

    let (_exit_code, production_json) = run_json(vec![
        "--json".to_owned(),
        "--danger".to_owned(),
        production_path.to_string_lossy().to_string(),
    ]);
    let (_exit_code, test_json) = run_json(vec![
        "--json".to_owned(),
        "--danger".to_owned(),
        test_path.to_string_lossy().to_string(),
    ]);
    let (_exit_code, nox_json) = run_json(vec![
        "--json".to_owned(),
        "--danger".to_owned(),
        nox_path.to_string_lossy().to_string(),
    ]);

    let test_aware_rules = [
        cytoscnpy::rules::ids::RULE_ID_EVAL,
        cytoscnpy::rules::ids::RULE_ID_EXEC,
        cytoscnpy::rules::ids::RULE_ID_ASSERT,
        cytoscnpy::rules::ids::RULE_ID_HARDCODED_CREDS,
        cytoscnpy::rules::ids::RULE_ID_RACE_CONDITION,
    ];
    for rule_id in test_aware_rules {
        assert!(
            has_rule(&production_json, "danger", rule_id),
            "Expected {rule_id} in production file"
        );
        assert!(
            !has_rule(&test_json, "danger", rule_id),
            "Did not expect {rule_id} in tests/test_app.py"
        );
        assert!(
            !has_rule(&nox_json, "danger", rule_id),
            "Did not expect {rule_id} in noxfile.py"
        );
    }

    assert!(has_rule(
        &test_json,
        "danger",
        cytoscnpy::rules::ids::RULE_ID_MD5
    ));
    assert!(has_rule(
        &nox_json,
        "danger",
        cytoscnpy::rules::ids::RULE_ID_MD5
    ));
}

#[test]
fn test_quality_rules_still_report_in_test_files() {
    let dir = project_tempdir();
    let tests_dir = dir.path().join("tests");
    fs::create_dir(&tests_dir).expect("Failed to create tests dir");
    let file_path = tests_dir.join("test_quality.py");
    fs::write(
        &file_path,
        r"
def bad(values=[]):
    try:
        if True == flag:
            return values
    except:
        return []
",
    )
    .expect("Failed to write test file");

    let (_exit_code, json) = run_json(vec![
        "--json".to_owned(),
        "--quality".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);

    for rule_id in [
        cytoscnpy::rules::ids::RULE_ID_MUTABLE_DEFAULT,
        cytoscnpy::rules::ids::RULE_ID_BARE_EXCEPT,
        cytoscnpy::rules::ids::RULE_ID_DANGEROUS_COMPARISON,
    ] {
        assert!(
            has_rule(&json, "quality", rule_id),
            "Expected quality rule {rule_id} in test file"
        );
    }
}
