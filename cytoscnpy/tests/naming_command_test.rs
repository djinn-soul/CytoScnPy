//! Integration tests for the `naming` CLI subcommand and unified deslop integration.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_cli_naming_clean_project() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("__init__.py"), "").unwrap();
    fs::write(
        src.join("alpha.py"),
        "def compute_alpha():\n    return 1\n\ndef _internal_alpha():\n    return 2\n",
    )
    .unwrap();
    fs::write(src.join("beta.py"), "def compute_beta():\n    return 3\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["naming".to_owned(), root.to_string_lossy().into_owned()],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Naming Style Distribution & Consistency Analysis"));
    assert!(output_str.contains("Total Definitions Analyzed"));
    assert!(output_str.contains("snake_case"));
    assert!(output_str.contains("Dominant"));
}

#[test]
fn test_cli_naming_json_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("worker.py"),
        "def handle_job():\n    pass\n\nclass Worker:\n    def __init__(self):\n        pass\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "naming".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    assert!(parsed.get("stats").is_some());
    assert_eq!(parsed["stats"]["total_identifiers"], 2);
    assert_eq!(parsed["stats"]["dunder_count"], 1);
    assert_eq!(parsed["stats"]["snake_case_count"], 1);
    assert_eq!(parsed["stats"]["pascal_case_count"], 1);
    assert_eq!(parsed["stats"]["dominant_style"], "snake_case");
    assert_eq!(parsed["stats"]["dominant_style_ratio"], 1.0);
    assert!(parsed["outliers"].as_array().unwrap().is_empty());
}

#[test]
fn test_cli_naming_global_json_and_output_file() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let out_file = temp.path().join("naming_report.json");

    fs::write(root.join("worker.py"), "def run_task():\n    pass\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "--json".to_owned(),
            "naming".to_owned(),
            root.to_string_lossy().into_owned(),
            "-o".to_owned(),
            out_file.to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    assert!(out_file.exists());
    let content = fs::read_to_string(&out_file).unwrap();
    assert!(content.contains("\"dominant_style\": \"snake_case\""));
}

#[test]
fn test_cli_naming_outliers_and_gating() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // 2 snake_case, 2 camelCase -> ratio 50%
    fs::write(
        root.join("mixed_repo.py"),
        r"
def first_task():
    pass

def second_task():
    pass

def badTaskOne():
    pass

def badTaskTwo():
    pass
",
    )
    .unwrap();

    // Normal invocation exits 0
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["naming".to_owned(), root.to_string_lossy().into_owned()],
        &mut out,
    )
    .unwrap();
    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("badTaskOne"));
    assert!(output_str.contains("badTaskTwo"));

    // With --fail-on-inconsistent (default 0.85), ratio is 0.50 -> exit 1
    let mut out_fail = Cursor::new(Vec::new());
    let code_fail = entry_point::run_with_args_to(
        vec![
            "naming".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-inconsistent".to_owned(),
        ],
        &mut out_fail,
    )
    .unwrap();
    assert_eq!(code_fail, 1);
    let fail_output = String::from_utf8(out_fail.into_inner()).unwrap();
    assert!(fail_output.contains("[GATE] Naming consistency: 50.0%"));
    assert!(fail_output.contains("FAILED"));

    // With --min-consistency 0.40 --fail-on-inconsistent -> ratio 0.50 >= 0.40 -> exit 0
    let mut out_pass = Cursor::new(Vec::new());
    let code_pass = entry_point::run_with_args_to(
        vec![
            "naming".to_owned(),
            root.to_string_lossy().into_owned(),
            "--min-consistency".to_owned(),
            "0.40".to_owned(),
            "--fail-on-inconsistent".to_owned(),
        ],
        &mut out_pass,
    )
    .unwrap();
    assert_eq!(code_pass, 0);

    // With --min-consistency 40.0 (percentage form) -> normalized to 0.40 -> exit 0
    let mut out_pct_pass = Cursor::new(Vec::new());
    let code_pct_pass = entry_point::run_with_args_to(
        vec![
            "naming".to_owned(),
            root.to_string_lossy().into_owned(),
            "--min-consistency".to_owned(),
            "40.0".to_owned(),
            "--fail-on-inconsistent".to_owned(),
        ],
        &mut out_pct_pass,
    )
    .unwrap();
    assert_eq!(code_pct_pass, 0);

    // With --min-consistency 80.0 (percentage form) -> normalized to 0.80 -> ratio 0.50 < 0.80 -> exit 1
    let mut out_pct_fail = Cursor::new(Vec::new());
    let code_pct_fail = entry_point::run_with_args_to(
        vec![
            "naming".to_owned(),
            root.to_string_lossy().into_owned(),
            "--min-consistency".to_owned(),
            "80.0".to_owned(),
            "--fail-on-any".to_owned(),
        ],
        &mut out_pct_fail,
    )
    .unwrap();
    assert_eq!(code_pct_fail, 1);
}

#[test]
fn test_cli_deslop_unified_includes_naming() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"[project]
name = "sample"
version = "0.1.0"
"#,
    )
    .unwrap();
    fs::write(root.join("app.py"), "def start_engine(): pass\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let report: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    assert!(report.get("naming").is_some());
    assert_eq!(report["naming"]["stats"]["total_identifiers"], 1);
    assert_eq!(report["naming"]["stats"]["dominant_style"], "snake_case");
    assert_eq!(report["naming"]["stats"]["dominant_style_ratio"], 1.0);
}

#[test]
fn test_cli_deslop_min_naming_consistency_gate() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"[project]
name = "sample"
version = "0.1.0"

[tool.cytoscnpy.deslop]
min_naming_consistency = 0.90
"#,
    )
    .unwrap();
    // 1 snake, 1 camel -> ratio 0.50 < 0.90
    fs::write(
        root.join("app.py"),
        "def start_engine(): pass\ndef runTask(): pass\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-any".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("naming_consistency: 50.0% (limit >= 90.0%)"));
}

#[test]
fn test_cli_naming_detects_class_naming_outlier() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("models.py"),
        "class bad_user_model:\n    pass\n\nclass GoodOrder:\n    pass\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "naming".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    assert_eq!(parsed["stats"]["total_identifiers"], 2);
    assert_eq!(parsed["stats"]["pascal_case_count"], 1);
    assert_eq!(parsed["stats"]["snake_case_count"], 1);
    assert_eq!(parsed["stats"]["dominant_style_ratio"], 0.5);

    let outliers = parsed["outliers"].as_array().unwrap();
    assert_eq!(outliers.len(), 1);
    assert_eq!(outliers[0]["name"], "bad_user_model");
    assert_eq!(outliers[0]["detected_style"], "snake_case");
    assert_eq!(outliers[0]["expected_style"], "pascal_case");
}
