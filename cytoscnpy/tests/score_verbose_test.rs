//! Integration tests for `--verbose` complete dimension output in `cytoscnpy score`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;
use cytoscnpy::scoring::{format_terminal_report, DimensionScore, ScoreResult, Verdict};

#[test]
fn test_cli_score_default_vs_verbose_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("sample.py"),
        r#"
GLOBAL_CONFIG = {"key": "value"}

def calculate(x, y):
    try:
        return x + y
    except:
        return 0
"#,
    )
    .unwrap();

    // 1. Default non-verbose run
    let mut out_default = Cursor::new(Vec::new());
    let code_default = entry_point::run_with_args_to(
        vec!["score".to_owned(), root.to_string_lossy().into_owned()],
        &mut out_default,
    )
    .unwrap();
    assert_eq!(code_default, 0);
    let str_default = String::from_utf8(out_default.into_inner()).unwrap();

    assert!(str_default.contains("Dimension Breakdown:\n"));
    assert!(!str_default.contains("Dimension Breakdown (Complete):\n"));
    assert!(!str_default.contains("Health Diagnostics Summary:"));
    assert!(!str_default.contains("-> Health Diagnostic:"));

    // 2. Verbose run with --verbose
    let mut out_verbose = Cursor::new(Vec::new());
    let code_verbose = entry_point::run_with_args_to(
        vec![
            "score".to_owned(),
            root.to_string_lossy().into_owned(),
            "--verbose".to_owned(),
        ],
        &mut out_verbose,
    )
    .unwrap();
    assert_eq!(code_verbose, 0);
    let str_verbose = String::from_utf8(out_verbose.into_inner()).unwrap();

    assert!(str_verbose.contains("Dimension Breakdown (Complete):\n"));
    assert!(str_verbose.contains("Health Diagnostics Summary:"));
    assert!(str_verbose.contains("-> Health Diagnostic:"));
    assert!(str_verbose.contains("Dimensions Analyzed : 10 total"));

    // 3. Short flag -v
    let mut out_short_v = Cursor::new(Vec::new());
    let code_short_v = entry_point::run_with_args_to(
        vec![
            "score".to_owned(),
            root.to_string_lossy().into_owned(),
            "-v".to_owned(),
        ],
        &mut out_short_v,
    )
    .unwrap();
    assert_eq!(code_short_v, 0);
    let str_short_v = String::from_utf8(out_short_v.into_inner()).unwrap();
    assert!(str_short_v.contains("Dimension Breakdown (Complete):\n"));
    assert!(str_short_v.contains("Health Diagnostics Summary:"));
}

#[test]
fn test_verbose_prioritizes_friction_dimensions() {
    let dimensions = vec![
        DimensionScore {
            name: "Setup reliability".to_owned(),
            weight: 10,
            rating: 0,
            raw_contribution: 0.0,
            evidence: "lockfile present".to_owned(),
        },
        DimensionScore {
            name: "Runtime predictability".to_owned(),
            weight: 10,
            rating: 4,
            raw_contribution: 8.0,
            evidence: "4 bare exceptions".to_owned(),
        },
        DimensionScore {
            name: "Architecture clarity".to_owned(),
            weight: 15,
            rating: 1,
            raw_contribution: 3.0,
            evidence: "1 duplicate file".to_owned(),
        },
    ];

    let result = ScoreResult {
        slop_index: 25,
        raw_score: 11.0,
        size_multiplier: 1.0,
        verdict: Verdict::Acceptable,
        dimensions,
        recommendations: vec![],
        summary: None,
        passed_gate: true,
        failure_reason: None,
    };

    let report_verbose = format_terminal_report(&result, true);

    // In verbose mode, Runtime predictability (8.0 pts) appears before Architecture clarity (3.0 pts)
    // and Setup reliability (0.0 pts) appears last among the three
    let pos_runtime = report_verbose
        .find("Runtime predictability")
        .expect("Runtime predictability should be present");
    let pos_arch = report_verbose
        .find("Architecture clarity")
        .expect("Architecture clarity should be present");
    let pos_setup = report_verbose
        .find("Setup reliability")
        .expect("Setup reliability should be present");

    assert!(pos_runtime < pos_arch);
    assert!(pos_arch < pos_setup);

    // Diagnostics should indicate specific status tags
    assert!(report_verbose.contains("[HIGH RISK] Severe architectural hazard"));
    assert!(report_verbose.contains("[LOW] Minor deviations"));
    assert!(report_verbose.contains("[PRISTINE] No friction detected"));
}

#[test]
fn test_verbose_expands_recommendations_and_files() {
    let mut recs = Vec::new();
    for i in 1..=8 {
        recs.push(cytoscnpy::scoring::Recommendation {
            id: format!("rec-{i}"),
            title: format!("Fix item {i}"),
            dimension: "Quality".to_owned(),
            estimated_reduction: 2,
            target_rating: 0,
            effort: cytoscnpy::scoring::Effort::Low,
            description: format!("Description for {i}"),
            action_steps: vec!["Step 1".to_owned()],
            affected_files: vec![
                "f1.py".to_owned(),
                "f2.py".to_owned(),
                "f3.py".to_owned(),
                "f4.py".to_owned(),
                "f5.py".to_owned(),
            ],
        });
    }

    let result = ScoreResult {
        slop_index: 30,
        raw_score: 30.0,
        size_multiplier: 1.0,
        verdict: Verdict::Acceptable,
        dimensions: vec![],
        recommendations: recs,
        summary: None,
        passed_gate: true,
        failure_reason: None,
    };

    // Non-verbose: capped at 5 remediations, preview at 3 files
    let default_report = format_terminal_report(&result, false);
    assert!(default_report.contains("1. [-2 pts] Fix item 1"));
    assert!(default_report.contains("5. [-2 pts] Fix item 5"));
    assert!(!default_report.contains("6. [-2 pts] Fix item 6"));
    assert!(default_report.contains("f1.py, f2.py, f3.py (+2 more)"));

    // Verbose: shows up to 10 remediations, preview up to 5 files
    let verbose_report = format_terminal_report(&result, true);
    assert!(verbose_report.contains("1. [-2 pts] Fix item 1"));
    assert!(verbose_report.contains("5. [-2 pts] Fix item 5"));
    assert!(verbose_report.contains("6. [-2 pts] Fix item 6"));
    assert!(verbose_report.contains("8. [-2 pts] Fix item 8"));
    assert!(verbose_report.contains("f1.py, f2.py, f3.py, f4.py, f5.py"));
}
