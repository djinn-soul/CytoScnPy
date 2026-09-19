//! Integration tests for multi-path argument order independence and CI max-score gate.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_ci_honors_explicit_max_score_ceiling() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create a messy codebase with globals and anti-patterns
    fs::write(
        root.join("messy.py"),
        r#"
GLOBAL_ONE = [1, 2, 3]
GLOBAL_TWO = {"a": 1}
GLOBAL_THREE = set()

def execute():
    try:
        pass
    except:
        pass
"#,
    )
    .unwrap();

    // With --ci and explicit generous --max-score 100, should pass even if verdict is not Clean/Acceptable
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "score".to_owned(),
            root.to_string_lossy().into_owned(),
            "--ci".to_owned(),
            "--max-score".to_owned(),
            "100".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("[GATE PASSED]"));
}

#[test]
fn test_multipath_order_independence() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dir_a = root.join("dir_a");
    let dir_b = root.join("dir_b");
    fs::create_dir_all(&dir_a).unwrap();
    fs::create_dir_all(&dir_b).unwrap();

    // dir_a has a python module with functions
    fs::write(
        dir_a.join("alpha.py"),
        "def compute_alpha():\n    return 1 + 2 + 3\n",
    )
    .unwrap();

    // dir_b has README and another module
    fs::write(
        dir_b.join("README.md"),
        "# Dir B\n\nSetup instructions:\nRun pip install .\n",
    )
    .unwrap();
    fs::write(
        dir_b.join("beta.py"),
        "def compute_beta():\n    return 'beta'\n",
    )
    .unwrap();

    // Order 1: dir_a, dir_b
    let mut out1 = Cursor::new(Vec::new());
    let code1 = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            dir_a.to_string_lossy().into_owned(),
            dir_b.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out1,
    )
    .unwrap();
    assert_eq!(code1, 0);
    let json1: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out1.into_inner()).unwrap()).unwrap();

    // Order 2: dir_b, dir_a
    let mut out2 = Cursor::new(Vec::new());
    let code2 = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            dir_b.to_string_lossy().into_owned(),
            dir_a.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out2,
    )
    .unwrap();
    assert_eq!(code2, 0);
    let json2: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out2.into_inner()).unwrap()).unwrap();

    // Both argument orders must yield identical Slop Index and Verdict
    assert_eq!(json1["slop_index"], json2["slop_index"]);
    assert_eq!(json1["verdict"], json2["verdict"]);
    assert_eq!(json1["raw_score"], json2["raw_score"]);
}

#[test]
fn test_unified_fail_on_any_gate_alignment() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("pyproject.toml"),
        r"
[tool.cytoscnpy.deslop]
max_slop_index = 80
min_health_score = 0
",
    )
    .unwrap();

    fs::write(
        root.join("app.py"),
        "def run_app():\n    return 'running'\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-any".to_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    assert_eq!(json["gates"]["passed"], true);
    assert_eq!(json["scoring"]["passed_gate"], true);
}

#[test]
fn test_llm_report_unhealthy_empty_recommendations() {
    use cytoscnpy::scoring::{DimensionScore, ScoreResult, Verdict};

    let unhealthy = ScoreResult {
        slop_index: 23,
        raw_score: 23.0,
        size_multiplier: 1.0,
        verdict: Verdict::Acceptable,
        dimensions: vec![DimensionScore {
            name: "Style consistency".to_owned(),
            weight: 10,
            rating: 2,
            raw_contribution: 4.0,
            evidence: "naming inconsistencies".to_owned(),
        }],
        recommendations: vec![],
        passed_gate: true,
        failure_reason: None,
    };
    let md = cytoscnpy::scoring::format_llm_report(&unhealthy);
    assert!(md.contains("No automated remediation recommendations available"));
    assert!(!md.contains("pristine"));

    let pristine = ScoreResult {
        slop_index: 0,
        raw_score: 0.0,
        size_multiplier: 0.0,
        verdict: Verdict::Clean,
        dimensions: vec![DimensionScore {
            name: "Style consistency".to_owned(),
            weight: 10,
            rating: 0,
            raw_contribution: 0.0,
            evidence: "Clean".to_owned(),
        }],
        recommendations: vec![],
        passed_gate: true,
        failure_reason: None,
    };
    let pristine_md = cytoscnpy::scoring::format_llm_report(&pristine);
    assert!(pristine_md.contains("Codebase is in pristine condition"));
}

#[test]
fn test_resolve_doctor_target_consistency() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let sub = root.join("tests").join("sub");
    fs::create_dir_all(&sub).unwrap();
    let py_file = sub.join("sample.py");
    fs::write(&py_file, "def test_it():\n    assert True\n").unwrap();

    let resolved_abs = cytoscnpy::doctor::resolve_doctor_target(&py_file, root);
    let canonical_sub = sub.canonicalize().unwrap();
    assert_eq!(resolved_abs, canonical_sub);
}
