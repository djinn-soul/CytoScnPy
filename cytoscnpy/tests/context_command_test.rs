//! Integration tests for the `context` CLI subcommand.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_cli_context_clean_project() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let app_dir = root.join("app");
    fs::create_dir_all(&app_dir).unwrap();
    fs::write(app_dir.join("__init__.py"), "").unwrap();
    fs::write(app_dir.join("models.py"), "class User:\n    pass\n").unwrap();
    fs::write(
        app_dir.join("services.py"),
        "def get_user():\n    return 'user'\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "context".to_owned(),
            root.to_string_lossy().into_owned(),
            "--no-git".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Git Activity & Code Surface"));
    assert!(output_str.contains("LLM Context Navigation Budget"));
    assert!(output_str.contains("Total Scanned Files"));
}

#[test]
fn test_cli_context_json_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let pkg = root.join("pkg");
    fs::create_dir_all(&pkg).unwrap();
    fs::write(pkg.join("__init__.py"), "").unwrap();
    fs::write(
        pkg.join("calc.py"),
        "def calculate(a, b):\n    if a > b:\n        return a - b\n    return a + b\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "context".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
            "--no-git".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    assert_eq!(parsed["total_files"], 2);
    assert!(parsed["total_lines"].as_u64().unwrap() > 0);
    assert!(parsed.get("git_activity").is_some());
    assert!(parsed.get("token_budget").is_some());
    assert!(parsed.get("hotspots").is_some());
    assert_eq!(parsed["token_budget"]["usable_context"], 176_000);
}

#[test]
fn test_cli_context_custom_budget_and_months() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let pkg = root.join("pkg");
    fs::create_dir_all(&pkg).unwrap();
    fs::write(pkg.join("module.py"), "def run(): pass\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "context".to_owned(),
            root.to_string_lossy().into_owned(),
            "--context-budget".to_owned(),
            "64000".to_owned(),
            "--git-months".to_owned(),
            "2".to_owned(),
            "--json".to_owned(),
            "--no-git".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    assert_eq!(parsed["token_budget"]["usable_context"], 64_000);
}

#[test]
fn test_cli_context_hotspots_only() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let pkg = root.join("pkg");
    fs::create_dir_all(&pkg).unwrap();
    fs::write(pkg.join("util.py"), "x = 1\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "context".to_owned(),
            root.to_string_lossy().into_owned(),
            "--hotspots-only".to_owned(),
            "--no-git".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Churn-Complexity Hotspots"));
    assert!(!output_str.contains("Git Activity & Code Surface"));
}

#[test]
fn test_cli_context_fail_on_hotspots_clean() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let pkg = root.join("pkg");
    fs::create_dir_all(&pkg).unwrap();
    fs::write(pkg.join("clean.py"), "def clean(): return 42\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "context".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-hotspots".to_owned(),
            "--no-git".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    // With no Git churn, no severe hotspots exist -> code 0
    assert_eq!(code, 0);
}

#[test]
fn test_cli_context_git_months_zero_rejected() {
    let mut out = Cursor::new(Vec::new());
    let res = entry_point::run_with_args_to(
        vec![
            "context".to_owned(),
            ".".to_owned(),
            "--git-months".to_owned(),
            "0".to_owned(),
        ],
        &mut out,
    );

    let code = res.unwrap_or(2);
    assert!(code != 0);
    assert_ne!(code, 101);
}

#[test]
fn test_cli_context_individual_file_target_runs_git_analysis() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let init = std::process::Command::new("git")
        .args(["init"])
        .current_dir(root)
        .output();
    if init.is_err() || !init.unwrap().status.success() {
        return;
    }
    let _ = std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(root)
        .output();
    let _ = std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(root)
        .output();

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    let app = src.join("app.py");
    fs::write(&app, "def main(): pass\n").unwrap();

    let _ = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(root)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(root)
        .output();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "context".to_owned(),
            app.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let val: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    assert_eq!(val["git_activity"]["is_git_repo"], true);
    assert_eq!(val["git_activity"]["active_files"], 1);
}
