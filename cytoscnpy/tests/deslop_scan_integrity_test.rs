//! Unified report regression tests for scan completion and findings.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

fn report(root: &std::path::Path, extra: &[&str]) -> (i32, serde_json::Value) {
    let mut args = vec![
        "deslop".to_owned(),
        root.to_string_lossy().into_owned(),
        "--json".to_owned(),
        "--no-git".to_owned(),
    ];
    args.extend(extra.iter().map(|s| (*s).to_owned()));
    let mut output = Cursor::new(Vec::new());
    let code = cytoscnpy::entry_point::run_with_args_to(args, &mut output).unwrap();
    let value: serde_json::Value = serde_json::from_slice(output.get_ref()).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert!(
        value.get("scan_integrity").is_some(),
        "expected unified deslop schema"
    );
    (code, value)
}

#[test]
fn unified_report_keeps_findings_and_configured_gates() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("app.py"),
        "from os import *\nITEMS = []\n# TODO: remove this\ntry:\n    pass\nexcept:\n    pass\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("pyproject.toml"),
        "[tool.cytoscnpy.deslop]\nmax_global_mutables = 0\nmax_bare_excepts = 0\nmax_wildcard_imports = 0\nmax_todos = 0\n",
    )
    .unwrap();

    let (code, value) = report(temp.path(), &["--fail-on-any"]);
    assert_eq!(code, 1);
    assert_eq!(value["scan_integrity"]["complete"], true);
    assert!(value["globals"]["stats"]["total_globals"].as_u64().unwrap() > 0);
    assert!(
        value["exceptions"]["stats"]["bare_except_count"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(value["wildcards"]["stats"]["total"].as_u64().unwrap() > 0);
    let failures = value["gates"]["failures"].as_array().unwrap();
    for gate in [
        "global_mutables",
        "bare_excepts",
        "wildcard_imports",
        "todos",
    ] {
        assert!(
            failures.iter().any(|item| item["check"] == gate),
            "missing gate {gate}"
        );
    }
}

#[test]
fn malformed_source_is_reported_and_fails_even_without_fail_on_any() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("good.py"), "def run():\n    return 1\n").unwrap();
    fs::write(temp.path().join("invalid.py"), "def broken(:\n    pass\n").unwrap();

    let (code, value) = report(temp.path(), &[]);
    assert_eq!(code, 1);
    assert_eq!(value["scan_integrity"]["complete"], false);
    assert_eq!(value["scan_integrity"]["files_discovered"], 2);
    assert_eq!(value["scan_integrity"]["files_checked"], 1);
    assert_eq!(value["functions"]["files_scanned"], 1);
    assert_eq!(value["unreferenced"]["stats"]["total_scanned_files"], 1);
    assert_eq!(
        value["functions"]["scan_issues"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        value["unreferenced"]["scan_issues"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(value["scan_integrity"]["issues"][0]["path"]
        .as_str()
        .unwrap()
        .ends_with("invalid.py"));
    assert!(value["gates"]["failures"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["check"] == "scan_integrity"));
}

#[test]
fn non_utf8_source_is_reported_instead_of_silently_skipped() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("invalid.py"), [0xff, 0xfe]).unwrap();

    let (code, value) = report(temp.path(), &[]);
    assert_eq!(code, 1);
    assert_eq!(value["scan_integrity"]["complete"], false);
    assert_eq!(value["scan_integrity"]["files_checked"], 0);
    assert!(value["scan_integrity"]["issues"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("read error"));
    assert_eq!(value["functions"]["files_scanned"], 0);
    assert_eq!(value["unreferenced"]["stats"]["total_scanned_files"], 0);
}

#[test]
fn explicitly_ignored_bad_source_does_not_make_scan_incomplete() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("good.py"), "def run():\n    return 1\n").unwrap();
    fs::write(temp.path().join("invalid.py"), "def broken(:\n").unwrap();

    let (code, value) = report(temp.path(), &["--ignore", "invalid.py"]);
    assert_eq!(code, 0);
    assert_eq!(value["scan_integrity"]["complete"], true);
    assert_eq!(value["scan_integrity"]["files_discovered"], 1);
}

#[test]
fn shared_inventory_preserves_naming_and_searchability_results() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("api.py"),
        "class OrderService:\n    def processOrder(self):\n        return 1\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("workers.py"),
        "def process_order():\n    return 2\n",
    )
    .unwrap();

    let (_, value) = report(temp.path(), &[]);
    let roots = vec![temp.path().to_path_buf()];
    let naming = cytoscnpy::naming::analyze_naming(&roots, &[], false);
    let searchability = cytoscnpy::searchability::analyze_searchability(&roots, &[], false);
    assert_eq!(value["naming"], serde_json::to_value(naming).unwrap());
    assert_eq!(
        value["searchability"],
        serde_json::to_value(searchability).unwrap()
    );
}

fn text_report(root: &std::path::Path, extra: &[&str]) -> (i32, String) {
    let mut args = vec![
        "deslop".to_owned(),
        root.to_string_lossy().into_owned(),
        "--no-git".to_owned(),
    ];
    args.extend(extra.iter().map(|s| (*s).to_owned()));
    let mut output = Cursor::new(Vec::new());
    let code = cytoscnpy::entry_point::run_with_args_to(args, &mut output).unwrap();
    (code, String::from_utf8(output.into_inner()).unwrap())
}

fn write_malformed_project(temp: &TempDir) {
    fs::write(temp.path().join("good.py"), "def run():\n    return 1\n").unwrap();
    fs::write(temp.path().join("invalid.py"), "def broken(:\n    pass\n").unwrap();
}

#[test]
fn gate_failure_reason_names_failed_gates_instead_of_score() {
    let temp = TempDir::new().unwrap();
    write_malformed_project(&temp);

    let (_, value) = report(temp.path(), &[]);
    assert_eq!(value["scoring"]["passed_gate"], false);
    assert!(value["scoring"]["failure_reason"]
        .as_str()
        .unwrap()
        .contains("scan_integrity"));
    let reason = value["scan_integrity"]["issues"][0]["reason"]
        .as_str()
        .unwrap();
    assert!(reason.starts_with("Python parse error"), "{reason}");

    let (code, verbose) = text_report(temp.path(), &["--verbose"]);
    assert_eq!(code, 1);
    assert!(!verbose.contains("Score threshold exceeded"));
    assert!(verbose.contains("[GATE FAILED] 1 DeSlopify gate(s) failed: scan_integrity"));
}

#[test]
fn llm_report_explains_gate_failures_and_incomplete_scan() {
    let temp = TempDir::new().unwrap();
    write_malformed_project(&temp);

    let (code, llm) = text_report(temp.path(), &["--format", "llm"]);
    assert_eq!(code, 1);
    assert!(llm.contains("# CI Gates\nFAILED\n- scan_integrity"));
    assert!(llm.contains("# Incomplete Scan (1 of 2 files checked)"));
}

#[test]
fn header_box_rows_are_aligned() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("app.py"), "def run():\n    return 1\n").unwrap();

    let (_, text) = text_report(temp.path(), &[]);
    let rows: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with('|') || line.starts_with('+'))
        .take(7)
        .collect();
    assert_eq!(rows.len(), 7);
    for row in rows {
        assert_eq!(row.chars().count(), 75, "misaligned row: {row:?}");
    }
}

#[test]
fn multi_root_globals_target_is_not_the_first_root() {
    let temp = TempDir::new().unwrap();
    for dir in ["a", "b"] {
        fs::create_dir(temp.path().join(dir)).unwrap();
        fs::write(temp.path().join(dir).join("m.py"), "ITEMS = []\n").unwrap();
    }
    let mut args = vec!["deslop".to_owned()];
    args.extend(["a", "b"].map(|d| temp.path().join(d).to_string_lossy().into_owned()));
    args.extend(["--json".to_owned(), "--no-git".to_owned()]);
    let mut output = Cursor::new(Vec::new());
    cytoscnpy::entry_point::run_with_args_to(args, &mut output).unwrap();
    let value: serde_json::Value = serde_json::from_slice(output.get_ref()).unwrap();

    assert_eq!(value["globals"]["stats"]["total_globals"], 2);
    let target = std::path::Path::new(value["globals"]["target"].as_str().unwrap());
    assert!(!target.ends_with("a"), "{}", target.display());
}
