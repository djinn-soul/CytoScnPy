//! Regressions for review findings in fixes and quality gates.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use cytoscnpy::entry_point::run_with_args_to;
use serde_json::Value;
use std::fmt::Write as _;
use std::fs;

fn scan(root: &std::path::Path, flags: &[&str]) -> anyhow::Result<(i32, Value)> {
    let mut args = vec![root.to_string_lossy().into_owned(), "--json".into()];
    args.extend(flags.iter().map(|flag| (*flag).to_owned()));
    let mut output = Vec::new();
    let exit = run_with_args_to(args, &mut output)?;
    Ok((exit, serde_json::from_slice(&output)?))
}

#[test]
fn incomplete_scan_never_changes_sources() {
    let temp = tempfile::tempdir().unwrap();
    let source = "def referenced():\n    return 42\n";
    let file = temp.path().join("api.py");
    fs::write(&file, source).unwrap();
    fs::write(
        temp.path().join("broken.py"),
        "from api import referenced\ndef broken(:\n    referenced()\n",
    )
    .unwrap();
    for flags in [vec!["--fix", "--apply"], vec!["--fix"]] {
        let error = scan(temp.path(), &flags).unwrap_err();
        assert!(error.to_string().contains("scan incomplete"), "{error}");
        assert_eq!(fs::read_to_string(&file).unwrap(), source);
    }
}

#[test]
fn every_configured_quality_limit_enables_analysis_and_fails_its_gate() {
    let cases = [
        ("max-lines", "2", "CSP-C304", "def run():\n    x = 1\n    x += 1\n    return x\n"),
        ("max-args", "1", "CSP-C303", "def run(a, b, c):\n    return a + b + c\n"),
        ("max-nesting", "1", "CSP-Q302", "def run(a):\n    if a:\n        if a > 2:\n            if a > 3:\n                return a\n    return 0\n"),
    ];
    for (option, value, rule, source) in cases {
        for from_config in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            fs::write(temp.path().join("app.py"), source).unwrap();
            let flag = format!("--{option}");
            let flags = if from_config {
                fs::write(
                    temp.path().join("pyproject.toml"),
                    format!("[tool.cytoscnpy]\n{} = {value}\n", option.replace('-', "_")),
                )
                .unwrap();
                vec![]
            } else {
                vec![flag.as_str(), value]
            };
            let (exit, result) = scan(temp.path(), &flags).unwrap();
            assert_eq!(exit, 1, "{option}, config={from_config}: {result}");
            assert!(result["quality"]
                .as_array()
                .unwrap()
                .iter()
                .any(|finding| finding["rule_id"] == rule));
        }
    }
}

#[test]
fn healthy_files_cannot_hide_a_file_below_minimum_mi() {
    let temp = tempfile::tempdir().unwrap();
    let mut heavy = String::from("def heavy(x):\n");
    for i in 0..250 {
        writeln!(heavy, "    if x == {i}:\n        x += {i}").unwrap();
    }
    heavy.push_str("    return x\n");
    fs::write(temp.path().join("heavy.py"), heavy).unwrap();
    for i in 0..20 {
        fs::write(temp.path().join(format!("clean{i}.py")), "value = 1\n").unwrap();
    }
    let (exit, result) = scan(temp.path(), &["--min-mi", "40"]).unwrap();
    assert!(result["analysis_summary"]["average_mi"].as_f64().unwrap() > 40.0);
    assert_eq!(exit, 1);
}

#[test]
fn invalid_fail_thresholds_fail_even_for_empty_inventories() {
    for value in ["NaN", "inf", "101", "-1"] {
        let temp = tempfile::tempdir().unwrap();
        let flag = format!("--fail-threshold={value}");
        let error = scan(temp.path(), &[&flag]).unwrap_err();
        assert!(error
            .to_string()
            .contains("finite number between 0 and 100"));
    }
    for value in ["nan", "inf", "101", "-1"] {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("pyproject.toml"),
            format!("[tool.cytoscnpy]\nfail_threshold = {value}\n"),
        )
        .unwrap();
        assert!(scan(temp.path(), &[]).is_err());
    }
    for value in ["0", "100"] {
        let temp = tempfile::tempdir().unwrap();
        assert_eq!(
            scan(temp.path(), &["--fail-threshold", value]).unwrap().0,
            0
        );
    }
}
