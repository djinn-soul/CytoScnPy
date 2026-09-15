//! Integration tests for the `todos` CLI subcommand and deslop integration.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

// ---------------------------------------------------------------------------
// Basic invocation
// ---------------------------------------------------------------------------

#[test]
fn test_cli_todos_clean_project() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("engine.py"),
        "def compute_result(a, b):\n    return a + b\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["todos".to_owned(), root.to_string_lossy().into_owned()],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("TODO / Annotation Scan Results"));
    assert!(output_str.contains("No TODO/FIXME/HACK/XXX"));
}

#[test]
fn test_cli_todos_detects_placeholders() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("work.py"),
        "# TODO: implement this\n# FIXME: broken\nx = 1\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["todos".to_owned(), root.to_string_lossy().into_owned()],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0); // no --fail-on-any
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("TODO placeholder left in code"));
    assert!(output_str.contains("FIXME placeholder left in code"));
}

// ---------------------------------------------------------------------------
// JSON output
// ---------------------------------------------------------------------------

#[test]
fn test_cli_todos_json_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("service.py"),
        "# TODO: add retry logic\nprint('debug')\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "todos".to_owned(),
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
    assert!(parsed["stats"]["total"].as_u64().unwrap() >= 1);
    assert!(parsed["stats"]["todo_count"].as_u64().unwrap() >= 1);
    assert!(parsed.get("matches").is_some());
}

// ---------------------------------------------------------------------------
// fail-on-any gating
// ---------------------------------------------------------------------------

#[test]
fn test_cli_todos_fail_on_any_with_annotations() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("legacy.py"), "# HACK: remove this\nx = 1\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "todos".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-any".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("[GATE]"));
    assert!(output_str.contains("FAILED"));
}

#[test]
fn test_cli_todos_fail_on_any_clean_exits_0() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("clean.py"), "def add(a, b):\n    return a + b\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "todos".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-any".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
}

// ---------------------------------------------------------------------------
// File output
// ---------------------------------------------------------------------------

#[test]
fn test_cli_todos_output_file() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let out_file = temp.path().join("todos_report.json");

    fs::write(root.join("task.py"), "# TODO: refactor this\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "todos".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
            "-o".to_owned(),
            out_file.to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    assert!(out_file.exists());
    let content = fs::read_to_string(&out_file).unwrap();
    assert!(content.contains("\"todo_count\""));
}

// ---------------------------------------------------------------------------
// Deslop unified integration
// ---------------------------------------------------------------------------

#[test]
fn test_cli_deslop_unified_includes_todos() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("pyproject.toml"),
        "[project]\nname = \"sample\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(root.join("app.py"), "def start(): pass\n").unwrap();

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

    assert!(report.get("todos").is_some());
    assert!(report["todos"]["stats"]["total"].as_u64().is_some());
    assert!(report["todos"]["matches"].as_array().is_some());
}

#[test]
fn test_cli_deslop_max_todos_gate() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("pyproject.toml"),
        "[project]\nname = \"sample\"\nversion = \"0.1.0\"\n\n[tool.cytoscnpy.deslop]\nmax_todos = 0\n",
    )
    .unwrap();
    fs::write(root.join("app.py"), "# TODO: fix this\ndef run(): pass\n").unwrap();

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
    assert!(output_str.contains("todos"));
}
