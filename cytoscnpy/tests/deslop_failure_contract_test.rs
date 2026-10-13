//! Explicit unified CLI contracts for invalid paths and incomplete scans.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use cytoscnpy::entry_point::run_with_args_to;
use std::fs;

#[test]
fn nonexistent_deslop_paths_fail_without_panicking_or_fabricating_a_report() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp
        .path()
        .join("nonexistent")
        .to_string_lossy()
        .into_owned();
    for paths in [vec![missing.clone()], vec!["--root".into(), missing]] {
        let mut args = vec!["deslop".into()];
        args.extend(paths);
        args.push("--json".into());
        let mut output = Vec::new();
        assert_eq!(run_with_args_to(args, &mut output).unwrap(), 1);
        assert!(output.is_empty(), "expected empty, got {output:?}");
    }
}

#[test]
fn partial_deslop_reports_use_the_unified_schema_and_fail_without_a_gate_flag() {
    for content in [b"def broken(:\n".as_slice(), &[0xff, 0xfe]] {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("broken.py"), content).unwrap();
        fs::write(temp.path().join("good.py"), "def run():\n    return 1\n").unwrap();
        let mut output = Vec::new();
        let code = run_with_args_to(
            vec![
                "deslop".into(),
                temp.path().to_string_lossy().into_owned(),
                "--json".into(),
            ],
            &mut output,
        )
        .unwrap();
        assert_eq!(code, 1);
        let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(report["schema_version"], 1);
        assert_eq!(report["scan_integrity"]["complete"], false);
        assert_eq!(report["scan_integrity"]["files_discovered"], 2);
        assert_eq!(report["scan_integrity"]["files_checked"], 1);
        assert_eq!(
            report["scan_integrity"]["issues"].as_array().unwrap().len(),
            1
        );
        assert_eq!(report["gates"]["passed"], false);
        assert!(report.get("analysis").is_none());
    }
}

#[test]
fn deslop_fails_when_no_python_files_are_found() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("README.md"), "# docs\n").unwrap();
    let mut output = Vec::new();
    let code = run_with_args_to(
        vec![
            "deslop".into(),
            temp.path().to_string_lossy().into_owned(),
            "--json".into(),
            "--no-git".into(),
        ],
        &mut output,
    )
    .unwrap();
    assert_eq!(code, 1);
    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(report["scan_integrity"]["files_discovered"], 0);
    let failures = report["gates"]["failures"].as_array().unwrap();
    assert!(failures.iter().any(|f| f["check"] == "no_python_files"));
}

#[test]
fn explicit_deslop_format_overrides_json_flags() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("good.py"), "def run():\n    return 1\n").unwrap();
    let target = temp.path().to_string_lossy().into_owned();
    for args in [
        vec!["deslop", &target, "--json", "--format", "llm"],
        vec!["--json", "deslop", &target, "--format", "terminal"],
    ] {
        let mut args: Vec<String> = args.into_iter().map(Into::into).collect();
        args.push("--no-git".into());
        let mut output = Vec::new();
        run_with_args_to(args, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&text).is_err());
        assert!(text.contains("# CI Gates") || text.contains("SLOP INDEX"));
    }
}
