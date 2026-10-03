//! Gate regressions for incomplete scans and measured MI zero.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use cytoscnpy::entry_point::run_with_args_to;
use serde_json::Value;
use std::fmt::Write as _;

#[test]
fn parsing_failures_fail_even_without_explicit_failure_flags() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("broken.py");
    std::fs::write(&file, "def broken(:\n").unwrap();
    for flags in [
        vec![],
        vec!["--fail-on-any"],
        vec!["--danger", "--fail-on-danger"],
    ] {
        let mut args = vec![file.to_string_lossy().into_owned(), "--json".into()];
        args.extend(flags.into_iter().map(str::to_owned));
        let mut output = Vec::new();
        assert_eq!(run_with_args_to(args, &mut output).unwrap(), 1);
        let result: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(result["parse_errors"].as_array().unwrap().len(), 1);
    }
}

#[test]
fn partial_results_remain_available_when_scan_gate_fails() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("broken.py"), "def broken(:\n").unwrap();
    std::fs::write(dir.path().join("valid.py"), "def unused():\n    pass\n").unwrap();
    let mut output = Vec::new();
    assert_eq!(
        run_with_args_to(
            vec![dir.path().to_string_lossy().into_owned(), "--json".into()],
            &mut output
        )
        .unwrap(),
        1
    );
    let result: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(result["parse_errors"].as_array().unwrap().len(), 1);
    assert_eq!(result["unused_functions"].as_array().unwrap().len(), 1);
}

#[test]
fn measured_mi_zero_fails_a_positive_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("app.py");
    let mut decisions = String::new();
    for i in 0..250 {
        writeln!(decisions, "    if x == {i}:\n        x += {i}").unwrap();
    }
    std::fs::write(
        &file,
        format!("def heavy(x):\n{decisions}    return x\nprint(heavy(1))\n"),
    )
    .unwrap();
    for (threshold, expected_exit) in [("40", 1), ("0", 0)] {
        let mut output = Vec::new();
        let exit = run_with_args_to(
            vec![
                file.to_string_lossy().into_owned(),
                "--min-mi".into(),
                threshold.into(),
                "--json".into(),
            ],
            &mut output,
        )
        .unwrap();
        let result: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(result["analysis_summary"]["average_mi"], 0.0);
        assert_eq!(exit, expected_exit);
    }
}

#[test]
fn an_empty_inventory_is_not_a_measured_mi_zero() {
    let dir = tempfile::tempdir().unwrap();
    let mut output = Vec::new();
    assert_eq!(
        run_with_args_to(
            vec![
                dir.path().to_string_lossy().into_owned(),
                "--min-mi".into(),
                "40".into(),
                "--json".into()
            ],
            &mut output
        )
        .unwrap(),
        0
    );
}
