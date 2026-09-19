//! Integration tests for the `score` (Slop Index) subcommand.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_cli_score_clean_project() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("main.py"),
        "def main():\n    print('hello world')\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["score".to_owned(), root.to_string_lossy().into_owned()],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("DESLOPIFY SLOP INDEX REPORT"));
    assert!(output_str.contains("Slop Index"));
    assert!(output_str.contains("Raw Score"));
    assert!(output_str.contains("Size Multiplier"));
    assert!(output_str.contains("Dimension Breakdown:"));
    assert!(output_str.contains("Architecture clarity"));
    assert!(output_str.contains("Coupling / blast radius"));
    assert!(output_str.contains("Setup reliability"));
    assert!(output_str.contains("Test safety net"));
    assert!(output_str.contains("Runtime predictability"));
    assert!(output_str.contains("[GATE PASSED]"));
}

#[test]
fn test_cli_score_aliases() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("app.py"), "def run():\n    return 42\n").unwrap();

    for alias in ["slop-index", "slop"] {
        let mut out = Cursor::new(Vec::new());
        let code = entry_point::run_with_args_to(
            vec![alias.to_owned(), root.to_string_lossy().into_owned()],
            &mut out,
        )
        .unwrap();

        assert_eq!(code, 0);
        let output_str = String::from_utf8(out.into_inner()).unwrap();
        assert!(output_str.contains("DESLOPIFY SLOP INDEX REPORT"));
    }
}

#[test]
fn test_cli_score_json_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("core.py"), "def process():\n    pass\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "score".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    assert!(parsed.get("slop_index").is_some());
    assert!(parsed.get("verdict").is_some());
    assert!(parsed.get("raw_score").is_some());
    assert!(parsed.get("size_multiplier").is_some());
    assert!(parsed.get("dimensions").is_some());
    assert_eq!(parsed["dimensions"].as_array().unwrap().len(), 10);
    assert!(parsed.get("recommendations").is_some());
    assert_eq!(parsed["passed_gate"], true);
}

#[test]
fn test_cli_score_max_score_gate() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("sample.py"),
        "GLOBAL_STATE = [1, 2, 3]\n\ndef run():\n    return GLOBAL_STATE\n",
    )
    .unwrap();

    // Generous gate passes
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "score".to_owned(),
            root.to_string_lossy().into_owned(),
            "--max-score".to_owned(),
            "100".to_owned(),
        ],
        &mut out,
    )
    .unwrap();
    assert_eq!(code, 0);

    // Strict gate 0 fails when there are setup/runtime slop dimensions
    let mut out2 = Cursor::new(Vec::new());
    let code2 = entry_point::run_with_args_to(
        vec![
            "score".to_owned(),
            root.to_string_lossy().into_owned(),
            "--max-score".to_owned(),
            "0".to_owned(),
            "--fail-on-any".to_owned(),
        ],
        &mut out2,
    )
    .unwrap();
    assert_eq!(code2, 1);
    let output_str2 = String::from_utf8(out2.into_inner()).unwrap();
    assert!(output_str2.contains("[GATE FAILED]"));
}

#[test]
fn test_cli_score_ci_mode() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("app.py"), "def compute():\n    return 100\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "score".to_owned(),
            root.to_string_lossy().into_owned(),
            "--ci".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    // Small clean codebase has low slop index and passes CI
    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("[GATE PASSED]"));
}

#[test]
fn test_deslop_unified_includes_score() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("main.py"), "def entry():\n    return True\n").unwrap();

    // Test JSON output of unified deslop
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
    let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    assert!(parsed.get("slop_index").is_some());
    assert!(parsed.get("verdict").is_some());
    assert!(parsed.get("raw_score").is_some());
    assert!(parsed.get("size_multiplier").is_some());
    assert!(parsed.get("dimensions").is_some());
    assert_eq!(parsed["dimensions"].as_array().unwrap().len(), 10);
    assert!(parsed.get("recommendations").is_some());
    assert!(parsed.get("scoring").is_some());

    // Test terminal output of unified deslop includes header
    let mut out2 = Cursor::new(Vec::new());
    let code2 = entry_point::run_with_args_to(
        vec!["deslop".to_owned(), root.to_string_lossy().into_owned()],
        &mut out2,
    )
    .unwrap();

    assert_eq!(code2, 0);
    let term_str = String::from_utf8(out2.into_inner()).unwrap();
    assert!(term_str.contains("DESLOPIFY SLOP INDEX REPORT"));
    assert!(term_str.contains("Slop Index"));
}

#[test]
fn test_cli_score_format_flag() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("test_app.py"), "def test_foo(): assert True\n").unwrap();

    // Test --format json
    let mut out_json = Cursor::new(Vec::new());
    let code_json = entry_point::run_with_args_to(
        vec![
            "score".to_owned(),
            root.to_string_lossy().into_owned(),
            "--format".to_owned(),
            "json".to_owned(),
        ],
        &mut out_json,
    )
    .unwrap();
    assert_eq!(code_json, 0);
    let json_str = String::from_utf8(out_json.into_inner()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert!(parsed.get("slop_index").is_some());

    // Test --format terminal
    let mut out_term = Cursor::new(Vec::new());
    let code_term = entry_point::run_with_args_to(
        vec![
            "score".to_owned(),
            root.to_string_lossy().into_owned(),
            "--format".to_owned(),
            "terminal".to_owned(),
        ],
        &mut out_term,
    )
    .unwrap();
    assert_eq!(code_term, 0);
    let term_str = String::from_utf8(out_term.into_inner()).unwrap();
    assert!(term_str.contains("DESLOPIFY SLOP INDEX REPORT"));

    // Test --format llm
    let mut out_llm = Cursor::new(Vec::new());
    let code_llm = entry_point::run_with_args_to(
        vec![
            "score".to_owned(),
            root.to_string_lossy().into_owned(),
            "--format".to_owned(),
            "llm".to_owned(),
        ],
        &mut out_llm,
    )
    .unwrap();
    assert_eq!(code_llm, 0);
    let llm_str = String::from_utf8(out_llm.into_inner()).unwrap();
    assert!(llm_str.contains("# Codebase Remediation Plan (DeSlopify)"));
    assert!(llm_str.contains("Current Health Snapshot"));
}
