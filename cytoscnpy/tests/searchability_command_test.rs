//! Integration tests for the `searchability` CLI subcommand and unified deslop integration.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_cli_searchability_clean_project() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("__init__.py"), "").unwrap();
    fs::write(src.join("alpha.py"), "def compute_alpha():\n    return 1\n").unwrap();
    fs::write(src.join("beta.py"), "def compute_beta():\n    return 2\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "searchability".to_owned(),
            root.to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Codebase Searchability & Name Collision Analysis"));
    assert!(output_str.contains("Total Files Scanned"));
}

#[test]
fn test_cli_searchability_json_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("one.py"), "def unique_action():\n    pass\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "searchability".to_owned(),
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
    assert_eq!(parsed["stats"]["total_files"], 1);
    assert_eq!(parsed["stats"]["total_functions"], 1);
    assert_eq!(parsed["stats"]["duplicate_filenames"], 0);
    assert_eq!(parsed["stats"]["function_name_collisions"], 0);
}

#[test]
fn test_cli_searchability_global_json_and_output_file() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let out_file = temp.path().join("report.json");

    fs::write(root.join("one.py"), "def unique_action():\n    pass\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "--json".to_owned(),
            "searchability".to_owned(),
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
    assert!(content.contains("\"total_files\": 1"));
}

#[test]
fn test_cli_searchability_duplicate_filenames_detection() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dir_a = root.join("pkg_a");
    let dir_b = root.join("pkg_b");
    fs::create_dir_all(&dir_a).unwrap();
    fs::create_dir_all(&dir_b).unwrap();

    fs::write(dir_a.join("service.py"), "def run_a(): pass\n").unwrap();
    fs::write(dir_b.join("service.py"), "def run_b(): pass\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "searchability".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    assert_eq!(parsed["stats"]["duplicate_filenames"], 1);
    assert_eq!(parsed["stats"]["worst_duplicate_filename"][0], "service.py");
    assert_eq!(parsed["stats"]["worst_duplicate_filename"][1], 2);
    assert_eq!(parsed["duplicate_files"].as_array().unwrap().len(), 1);
}

#[test]
fn test_cli_searchability_fail_on_duplicates() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dir_a = root.join("a");
    let dir_b = root.join("b");
    fs::create_dir_all(&dir_a).unwrap();
    fs::create_dir_all(&dir_b).unwrap();

    fs::write(dir_a.join("worker.py"), "x = 1\n").unwrap();
    fs::write(dir_b.join("worker.py"), "x = 2\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "searchability".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-duplicates".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1, "Should exit with 1 when duplicate filenames are present and --fail-on-duplicates is passed");
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("[GATE] Duplicate filenames: 1 duplicate file(s) found - FAILED"));
}

#[test]
fn test_cli_searchability_function_collisions_and_gating() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dir_a = root.join("sub_a");
    let dir_b = root.join("sub_b");
    let dir_c = root.join("sub_c");
    fs::create_dir_all(&dir_a).unwrap();
    fs::create_dir_all(&dir_b).unwrap();
    fs::create_dir_all(&dir_c).unwrap();

    fs::write(dir_a.join("f1.py"), "def parse_token():\n    return 1\n").unwrap();
    fs::write(dir_b.join("f2.py"), "def parse_token():\n    return 2\n").unwrap();
    fs::write(dir_c.join("f3.py"), "def parse_token():\n    return 3\n").unwrap();

    // Verify detection in JSON output
    let mut out_json = Cursor::new(Vec::new());
    let code_json = entry_point::run_with_args_to(
        vec![
            "searchability".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out_json,
    )
    .unwrap();
    assert_eq!(code_json, 0);

    let parsed: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out_json.into_inner()).unwrap()).unwrap();
    assert_eq!(parsed["stats"]["function_name_collisions"], 1);
    assert_eq!(
        parsed["stats"]["worst_function_collision"][0],
        "parse_token"
    );
    assert_eq!(parsed["stats"]["worst_function_collision"][1], 3);

    // Verify fail-on-collisions
    let mut out_fail = Cursor::new(Vec::new());
    let code_fail = entry_point::run_with_args_to(
        vec![
            "searchability".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-collisions".to_owned(),
        ],
        &mut out_fail,
    )
    .unwrap();
    assert_eq!(code_fail, 1);
    let output_fail = String::from_utf8(out_fail.into_inner()).unwrap();
    assert!(output_fail
        .contains("[GATE] Function name collisions: 1 colliding function(s) found - FAILED"));

    // Verify fail-on-any
    let mut out_any = Cursor::new(Vec::new());
    let code_any = entry_point::run_with_args_to(
        vec![
            "searchability".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-any".to_owned(),
        ],
        &mut out_any,
    )
    .unwrap();
    assert_eq!(code_any, 1);
}

#[test]
fn test_cli_deslop_unified_includes_searchability() {
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

    assert!(report.get("searchability").is_some());
    assert_eq!(report["searchability"]["stats"]["total_files"], 1);
    assert_eq!(report["searchability"]["stats"]["total_functions"], 1);
}
