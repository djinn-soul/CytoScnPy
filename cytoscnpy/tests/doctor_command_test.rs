//! Integration tests for the `doctor` / `health` CLI subcommand.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_cli_doctor_full_project() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create pyproject.toml
    fs::write(
        root.join("pyproject.toml"),
        r#"[project]
name = "demo-pkg"
version = "0.1.0"
dependencies = ["requests>=2.28.0", "pydantic>=2.0"]

[tool.ruff]
line-length = 100

[tool.pytest.ini_options]
minversion = "7.0"

[tool.mypy]
strict = true
"#,
    )
    .unwrap();

    // Create .gitignore
    fs::write(root.join(".gitignore"), "*.pyc\n__pycache__/\n.venv/\n").unwrap();

    // Create uv.lock
    fs::write(root.join("uv.lock"), "version = 1\n").unwrap();

    // Create CI workflow
    let workflows = root.join(".github").join("workflows");
    fs::create_dir_all(&workflows).unwrap();
    fs::write(workflows.join("ci.yml"), "name: CI\non: [push]\n").unwrap();

    // Create source files
    let src_dir = root.join("src").join("demo");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("__init__.py"), "").unwrap();
    fs::write(
        src_dir.join("main.py"),
        "def run():\n    return 'running'\n",
    )
    .unwrap();

    // Create test files
    let tests_dir = root.join("tests");
    fs::create_dir_all(&tests_dir).unwrap();
    fs::write(
        tests_dir.join("test_main.py"),
        "def test_run():\n    assert True\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["doctor".to_owned(), root.to_string_lossy().into_owned()],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Detected Tooling & Configuration Inventory"));
    assert!(output_str.contains("Setup Reliability & Repository Health"));
    assert!(output_str.contains("Repository Structure & Language Volume"));
}

#[test]
fn test_cli_doctor_json_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join(".gitignore"), "target/\n").unwrap();
    fs::write(
        root.join("app.py"),
        "def hello():\n    print('hello world')\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "doctor".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    assert!(json.get("reliability").is_some());
    assert!(json["reliability"].get("score").is_some());
    assert!(json.get("configs").is_some());
    assert!(json.get("structure").is_some());
    assert_eq!(json["structure"]["source_files"], 1);
}

#[test]
fn test_cli_health_alias() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("script.py"), "print(1)\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "health".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    assert!(json.get("reliability").is_some());
    assert!(json["reliability"].get("score").is_some());
}

#[test]
fn test_cli_doctor_fail_on_missing_gate() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Empty project with only a bare python script (no gitignore, no lockfile, no ci, no tests)
    fs::write(root.join("sample.py"), "x = 42\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "doctor".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-missing".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("[GATE] Setup reliability check failed"));
}

#[test]
fn test_cli_doctor_fail_on_missing_pass() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join(".gitignore"), "target/\n").unwrap();
    fs::write(root.join("uv.lock"), "").unwrap();
    let wf = root.join(".github").join("workflows");
    fs::create_dir_all(&wf).unwrap();
    fs::write(wf.join("ci.yml"), "name: CI\n").unwrap();
    fs::write(root.join("pytest.ini"), "[pytest]\n").unwrap();
    fs::write(root.join(".ruff.toml"), "line-length = 88\n").unwrap();
    fs::write(root.join("README.md"), "# Project\n## Setup\nRun tests.\n").unwrap();
    fs::write(root.join("main.py"), "print('ready')\n").unwrap();
    let tests = root.join("tests");
    fs::create_dir_all(&tests).unwrap();
    fs::write(tests.join("test_main.py"), "assert True\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "doctor".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-missing".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
}

#[test]
fn test_cli_doctor_respects_exclusions() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let src = root.join("src");
    let vendor = root.join("vendor");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&vendor).unwrap();

    fs::write(src.join("main.py"), "print('hello')\n").unwrap();
    fs::write(vendor.join("lib.py"), "print('vendor')\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "doctor".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
            "--exclude".to_owned(),
            "vendor".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    assert_eq!(json["structure"]["total_files"], 1);
    assert_eq!(json["structure"]["source_files"], 1);
}

#[test]
fn test_cli_doctor_exclusions_do_not_overmatch() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("contests.py"), "print('contest')\n").unwrap();
    fs::write(src.join("library.py"), "print('library')\n").unwrap();

    let tests = root.join("tests");
    fs::create_dir_all(&tests).unwrap();
    fs::write(tests.join("test_app.py"), "assert True\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "doctor".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
            "--exclude".to_owned(),
            "tests/".to_owned(),
            "--exclude".to_owned(),
            "./lib".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    assert_eq!(json["structure"]["source_files"], 2);
    assert_eq!(json["structure"]["total_files"], 2);
}

#[test]
fn test_cli_doctor_multiple_targets_fail_on_missing() {
    let temp = TempDir::new().unwrap();
    let healthy = temp.path().join("healthy");
    let broken = temp.path().join("broken");
    fs::create_dir_all(&healthy).unwrap();
    fs::create_dir_all(&broken).unwrap();

    let wf = healthy.join(".github").join("workflows");
    fs::create_dir_all(&wf).unwrap();
    fs::write(wf.join("ci.yml"), "name: CI\n").unwrap();
    fs::write(healthy.join("pytest.ini"), "[pytest]\n").unwrap();
    fs::write(healthy.join(".ruff.toml"), "line-length = 88\n").unwrap();
    fs::write(healthy.join("README.md"), "# Setup\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "doctor".to_owned(),
            healthy.to_string_lossy().into_owned(),
            broken.to_string_lossy().into_owned(),
            "--fail-on-missing".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Repository Health:"));
    assert!(output_str.contains("Setup reliability check failed"));
}
