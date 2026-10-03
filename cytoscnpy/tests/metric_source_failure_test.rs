//! Metric commands must preserve discovery, read, and parse failures.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use cytoscnpy::entry_point::run_with_args_to;

#[test]
fn decoding_failures_never_become_perfect_or_empty_metrics() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("undecodable.py");
    std::fs::write(&file, [0xff, 0xfe, 0x00]).unwrap();
    for command in ["cc", "mi", "raw", "hal"] {
        let mut output = Vec::new();
        let error = run_with_args_to(
            vec![
                command.into(),
                file.to_string_lossy().into_owned(),
                "--json".into(),
            ],
            &mut output,
        )
        .unwrap_err();
        let details = format!("{error:#}");
        assert!(details.contains("undecodable.py"), "{details}");
        assert!(details.contains("UTF-8"), "{details}");
        assert!(output.is_empty());
    }
}

#[test]
fn parse_dependent_metrics_report_invalid_python() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("malformed.py");
    std::fs::write(&file, "def broken(:\n").unwrap();
    for command in ["cc", "mi", "hal"] {
        let mut output = Vec::new();
        let error = run_with_args_to(
            vec![
                command.into(),
                file.to_string_lossy().into_owned(),
                "--json".into(),
            ],
            &mut output,
        )
        .unwrap_err();
        let details = format!("{error:#}");
        assert!(details.contains("malformed.py"));
        assert!(details.contains("Failed to parse"));
        assert!(output.is_empty());
    }
    // Raw metrics describe source text and do not require syntactically valid Python.
    let mut output = Vec::new();
    assert_eq!(
        run_with_args_to(
            vec![
                "raw".into(),
                file.to_string_lossy().into_owned(),
                "--json".into()
            ],
            &mut output
        )
        .unwrap(),
        0
    );
    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(result[0]["loc"], 1);
}

#[test]
fn programmatic_metric_commands_report_missing_inputs() {
    use cytoscnpy::commands::{run_cc, run_hal, run_mi, run_raw, CcOptions, MiOptions};
    let dir = tempfile::tempdir().unwrap();
    let roots = [dir.path().join("missing.py")];
    assert!(run_cc(&roots, CcOptions::default(), &mut Vec::new()).is_err());
    assert!(run_mi(&roots, MiOptions::default(), &mut Vec::new()).is_err());
    assert!(run_hal(
        &roots,
        true,
        vec![],
        vec![],
        false,
        None,
        false,
        &mut Vec::new()
    )
    .is_err());
    assert!(run_raw(
        &roots,
        true,
        vec![],
        vec![],
        false,
        None,
        false,
        &mut Vec::new()
    )
    .is_err());
}

#[test]
fn valid_empty_python_still_has_defined_metrics() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("empty.py");
    std::fs::write(&file, "").unwrap();
    let mut output = Vec::new();
    assert_eq!(
        run_with_args_to(
            vec![
                "mi".into(),
                file.to_string_lossy().into_owned(),
                "--json".into()
            ],
            &mut output
        )
        .unwrap(),
        0
    );
    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(result[0]["mi"], 100.0);
}
