//! Integration tests for the `globals` CLI subcommand and deslop integration.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_cli_globals_clean_project() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("engine.py"),
        "MAX_RETRIES = 5\nNAME = 'service'\n\ndef compute(a, b):\n    return a + b\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["globals".to_owned(), root.to_string_lossy().into_owned()],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Mutable Global State Analysis"));
    assert!(output_str.contains("No mutable global state detected"));
}

#[test]
fn test_cli_globals_detects_mutable_state() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("cache.py"),
        "CACHE = {}\nITEMS = []\n\nclass Store:\n    shared = []\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["globals".to_owned(), root.to_string_lossy().into_owned()],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Module Collections (Python)"));
    assert!(output_str.contains("Class Variables (Python)"));
    assert!(output_str.contains("CACHE"));
    assert!(output_str.contains("ITEMS"));
    assert!(output_str.contains("Store::shared"));
}

#[test]
fn test_cli_globals_json_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("service.py"), "REGISTRY = {}\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "globals".to_owned(),
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
    assert_eq!(parsed["stats"]["total_globals"].as_u64().unwrap(), 1);
    assert_eq!(
        parsed["stats"]["module_collection_count"].as_u64().unwrap(),
        1
    );
    assert!(parsed.get("matches").is_some());
}

#[test]
fn test_cli_globals_fail_on_any_with_globals() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("state.py"), "LIST_DATA = []\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "globals".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-any".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("[GATE] 1 mutable global(s) detected"));
}

#[test]
fn test_cli_globals_fail_on_any_clean_exits_0() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("clean.py"), "SAFE_VAL = 42\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "globals".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-any".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
}

#[test]
fn test_cli_globals_max_globals_gate() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("store.py"), "A = []\nB = {}\n").unwrap();

    // Limit 5 -> passes (2 <= 5)
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "globals".to_owned(),
            root.to_string_lossy().into_owned(),
            "--max-globals".to_owned(),
            "5".to_owned(),
        ],
        &mut out,
    )
    .unwrap();
    assert_eq!(code, 0);

    // Limit 1 -> fails (2 > 1)
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "globals".to_owned(),
            root.to_string_lossy().into_owned(),
            "--max-globals".to_owned(),
            "1".to_owned(),
        ],
        &mut out,
    )
    .unwrap();
    assert_eq!(code, 1);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("[GATE] 2 mutable global(s) detected (limit: 1) - FAILED"));
}

#[test]
fn test_cli_globals_output_file() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("data.py"), "REG = {}\n").unwrap();
    let out_file = root.join("report.json");

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "globals".to_owned(),
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
    assert!(content.contains(r#""total_globals": 1"#));
}

#[test]
fn test_cli_deslop_unified_includes_globals() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("app.py"), "GLOBAL_LIST = []\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
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

    assert!(parsed.get("globals").is_some());
    assert_eq!(
        parsed["globals"]["stats"]["total_globals"]
            .as_u64()
            .unwrap(),
        1
    );
    assert_eq!(
        parsed["globals"]["stats"]["module_collection_count"]
            .as_u64()
            .unwrap(),
        1
    );
}

#[test]
fn test_cli_deslop_max_global_mutables_gate() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("pyproject.toml"),
        "[tool.cytoscnpy.deslop]\nmax_global_mutables = 0\nmin_health_score = 0\n",
    )
    .unwrap();
    fs::write(root.join("app.py"), "CACHE = {}\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-any".to_owned(),
            "--no-git".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("global_mutables: 1 (limit <= 0)"));
}
