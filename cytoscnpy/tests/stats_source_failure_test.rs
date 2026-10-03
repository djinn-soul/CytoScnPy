//! Statistics must not turn source/discovery failures into successful empty reports.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use cytoscnpy::{
    commands::{run_files, run_stats_v2, ScanOptions},
    config::Config,
    entry_point::run_with_args_to,
};

#[test]
fn stats_and_files_report_decode_errors_without_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("undecodable.py");
    std::fs::write(&file, [0xff, 0xfe]).unwrap();
    for command in ["stats", "files"] {
        let mut output = Vec::new();
        let error = run_with_args_to(
            vec![
                command.into(),
                file.to_string_lossy().into_owned(),
                "--json".into(),
            ],
            &mut output,
        )
        .err()
        .unwrap();
        let details = format!("{error:#}");
        assert!(details.contains("undecodable.py"), "{details}");
        assert!(details.contains("UTF-8"), "{details}");
        assert!(output.is_empty());
    }
}

#[test]
fn stats_reports_syntax_failure_while_files_counts_readable_text() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("broken.py");
    std::fs::write(&file, "def broken(:\n").unwrap();
    let mut output = Vec::new();
    let error = run_with_args_to(
        vec![
            "stats".into(),
            file.to_string_lossy().into_owned(),
            "--json".into(),
        ],
        &mut output,
    )
    .err()
    .unwrap();
    assert!(format!("{error:#}").contains("Failed to parse Python source"));
    assert!(output.is_empty());
    assert_eq!(
        run_with_args_to(
            vec![
                "files".into(),
                file.to_string_lossy().into_owned(),
                "--json".into()
            ],
            &mut output
        )
        .unwrap(),
        0
    );
    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(result[0]["total_lines"], 1);
}

#[test]
fn programmatic_stats_and_files_propagate_discovery_failures() {
    let dir = tempfile::tempdir().unwrap();
    let paths = [dir.path().join("missing.py")];
    let mut output = Vec::new();
    assert!(run_files(&paths, true, &[], false, &mut output).is_err());
    assert!(run_stats_v2(
        dir.path(),
        &paths,
        ScanOptions {
            json: true,
            ..Default::default()
        },
        None,
        &[],
        false,
        &[],
        false,
        Config::default(),
        &mut output
    )
    .is_err());
    assert!(output.is_empty());
}

#[test]
fn valid_project_counts_are_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("app.py");
    std::fs::write(
        &file,
        "class Example:\n    def method(self):\n        pass\n\ndef function():\n    pass\n",
    )
    .unwrap();
    let mut output = Vec::new();
    assert_eq!(
        run_with_args_to(
            vec![
                "stats".into(),
                file.to_string_lossy().into_owned(),
                "--json".into()
            ],
            &mut output
        )
        .unwrap(),
        0
    );
    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(result["total_files"], 1);
    assert_eq!(result["total_functions"], 2);
    assert_eq!(result["total_classes"], 1);
    assert_eq!(result["total_lines"], 6);
}
