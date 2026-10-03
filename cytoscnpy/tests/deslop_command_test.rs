//! Integration tests for the unified `DeSlopify` command.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

fn project_fixture() -> TempDir {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("app.py"),
        "def greet(name):\n    return f'Hello {name}'\n",
    )
    .unwrap();
    temp
}

#[test]
fn returns_one_report_with_every_analysis_section() {
    let temp = project_fixture();
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            temp.path().to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let report: serde_json::Value =
        serde_json::from_slice(&out.into_inner()).expect("report must be valid JSON");
    assert_eq!(report["schema_version"], 1);
    for section in [
        "slop_index",
        "dimensions",
        "recommendations",
        "architecture",
        "context",
        "health",
        "searchability",
        "naming",
        "todos",
        "globals",
        "exceptions",
        "wildcards",
        "side_effects",
        "singletons",
        "anti_patterns",
        "duplicates",
        "unreferenced",
        "functions",
        "gates",
    ] {
        assert!(report.get(section).is_some(), "missing {section}");
    }
    assert!(report.get("analysis").is_none());
    assert_eq!(report["gates"]["passed"], true);
}

#[test]
fn default_text_stops_after_slop_index_gate() {
    let temp = project_fixture();
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            temp.path().to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let report = String::from_utf8(out.into_inner()).unwrap();
    assert!(report.contains("DESLOPIFY SLOP INDEX REPORT"));
    assert!(report.contains("Dimension Breakdown:"));
    assert!(report
        .trim_end()
        .ends_with("[GATE PASSED] Codebase satisfies slop index and threshold requirements."));
    assert!(!report.contains("# Architecture"));
    assert!(!report.contains("Duplicate Clusters (Top"));
    assert!(!report.contains("# Python Function Metrics"));
}

#[test]
fn default_text_reports_configured_gate_failures() {
    let temp = project_fixture();
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            temp.path().to_string_lossy().into_owned(),
            "--fail-on-any".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let report = String::from_utf8(out.into_inner()).unwrap();
    assert!(report.contains("[GATE FAILED]"));
    assert!(report.contains("# CI Gates\nFAILED"));
    assert!(!report.contains("[GATE PASSED]"));
    assert!(!report.contains("# Architecture"));
}

#[test]
fn verbose_text_includes_full_analysis_sections() {
    let temp = project_fixture();
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            temp.path().to_string_lossy().into_owned(),
            "--verbose".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let report = String::from_utf8(out.into_inner()).unwrap();
    assert!(report.contains("DESLOPIFY SLOP INDEX REPORT"));
    assert!(report.contains("# Architecture"));
    assert!(report.contains("# Duplicate Code Clusters"));
    assert!(report.contains("# Python Function Metrics"));
    assert!(report.contains("# CI Gates"));
}

#[test]
fn local_fail_on_any_runs_deslop_checks_and_returns_failure() {
    let temp = project_fixture();
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            temp.path().to_string_lossy().into_owned(),
            "--json".to_owned(),
            "--fail-on-any".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let report: serde_json::Value =
        serde_json::from_slice(&out.into_inner()).expect("report must be valid JSON");
    assert_eq!(report["gates"]["passed"], false);
    assert!(!report["gates"]["failures"].as_array().unwrap().is_empty());
    assert!(report.get("analysis").is_none());
}

#[test]
fn global_fail_on_any_uses_the_same_comprehensive_report() {
    let temp = project_fixture();
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "--fail-on-any".to_owned(),
            "deslop".to_owned(),
            temp.path().to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let report: serde_json::Value =
        serde_json::from_slice(&out.into_inner()).expect("report must be valid JSON");
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["gates"]["passed"], false);
}

#[test]
fn global_json_flag_returns_the_unified_json_report() {
    let temp = project_fixture();
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "--json".to_owned(),
            "deslop".to_owned(),
            temp.path().to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let report: serde_json::Value =
        serde_json::from_slice(&out.into_inner()).expect("report must be valid JSON");
    assert_eq!(report["schema_version"], 1);
}

#[test]
fn ci_uses_configured_score_ceiling() {
    let temp = project_fixture();
    fs::write(
        temp.path().join("app.py"),
        "GLOBAL_STATE = []\n\ndef run():\n    try:\n        return GLOBAL_STATE\n    except:\n        pass\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("pyproject.toml"),
        "[tool.cytoscnpy.deslop]\nmax_slop_index = 0\n",
    )
    .unwrap();
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            temp.path().to_string_lossy().into_owned(),
            "--ci".to_owned(),
            "--json".to_owned(),
            "--no-git".to_owned(),
            "--context-budget".to_owned(),
            "1".to_owned(),
        ],
        &mut out,
    )
    .unwrap();
    let report: serde_json::Value = serde_json::from_slice(out.get_ref()).unwrap();
    assert_eq!(code, 1, "slop index: {}", report["slop_index"]);
    assert!(report["gates"]["failures"]
        .as_array()
        .unwrap()
        .iter()
        .any(|failure| failure["check"] == "slop_index"));
}

#[test]
fn fail_on_any_gates_circular_dependencies() {
    let temp = TempDir::new().unwrap();
    let package = temp.path().join("pkg");
    fs::create_dir(&package).unwrap();
    fs::write(package.join("__init__.py"), "").unwrap();
    fs::write(package.join("a.py"), "from pkg.b import value\n").unwrap();
    fs::write(package.join("b.py"), "from pkg.a import value\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            temp.path().to_string_lossy().into_owned(),
            "--json".to_owned(),
            "--fail-on-any".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let report: serde_json::Value =
        serde_json::from_slice(&out.into_inner()).expect("report must be valid JSON");
    assert_eq!(
        report["architecture"]["stats"]["circular_dependency_count"],
        1
    );
    assert!(report["gates"]["failures"]
        .as_array()
        .unwrap()
        .iter()
        .any(|failure| failure["check"] == "circular_dependencies"));
}
