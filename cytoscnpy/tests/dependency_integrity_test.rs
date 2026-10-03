//! Dependency results retain errors and do not infer absence from incomplete scans.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use cytoscnpy::entry_point::run_with_args_to;
use serde_json::Value;
use std::{fs, path::Path};

fn scan(root: &Path, extra: &[&str]) -> (i32, Value) {
    let mut args = vec![
        "deps".into(),
        root.to_string_lossy().into_owned(),
        "--json".into(),
    ];
    args.extend(extra.iter().map(|arg| (*arg).to_owned()));
    let mut output = Vec::new();
    let code = run_with_args_to(args, &mut output).unwrap();
    (code, serde_json::from_slice(&output).unwrap())
}

fn manifest(root: &Path, dependencies: &str) {
    fs::write(
        root.join("pyproject.toml"),
        format!("[project]\nname='example'\nversion='0.1'\ndependencies=[{dependencies}]\n"),
    )
    .unwrap();
}

#[test]
fn parse_and_read_errors_fail_scans_without_false_removal_recommendations() {
    for source in [b"import requests\ndef broken(:\n".as_slice(), &[0xff, 0xfe]] {
        let dir = tempfile::tempdir().unwrap();
        manifest(dir.path(), "'requests'");
        fs::write(dir.path().join("broken.py"), source).unwrap();
        fs::write(dir.path().join("valid.py"), "import another_dependency\n").unwrap();
        let (code, result) = scan(dir.path(), &[]);
        assert_eq!(code, 1);
        assert_eq!(result["scan_complete"], false);
        assert_eq!(result["scan_errors"].as_array().unwrap().len(), 1);
        assert!(result["scan_errors"][0]["file"]
            .as_str()
            .unwrap()
            .ends_with("broken.py"));
        assert!(result["unused"].as_array().unwrap().is_empty());
        assert!(result["removable_branches"].as_array().unwrap().is_empty());
        assert_eq!(result["missing"], serde_json::json!(["another_dependency"]));
        assert!(!result["missing_details"][0]["locations"]
            .as_array()
            .unwrap()
            .is_empty());
    }
}

#[test]
fn malformed_dependency_source_cannot_pass_missing_gate() {
    let dir = tempfile::tempdir().unwrap();
    manifest(dir.path(), "");
    let file = dir.path().join("app.py");
    fs::write(&file, "import requests\n").unwrap();
    assert_eq!(scan(dir.path(), &["--fail-on-missing"]).0, 1);
    fs::write(&file, "import requests\ndef broken(:\n").unwrap();
    let (code, result) = scan(dir.path(), &["--fail-on-missing"]);
    assert_eq!(code, 1);
    assert!(!result["scan_errors"].as_array().unwrap().is_empty());
    let mut output = Vec::new();
    assert_eq!(
        run_with_args_to(
            vec!["deps".into(), dir.path().to_string_lossy().into_owned()],
            &mut output
        )
        .unwrap(),
        1
    );
    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("Dependency scan incomplete"));
    assert!(!text.contains("No unused, missing"));
}

#[test]
fn requested_and_included_requirements_errors_retain_paths_and_context() {
    let dir = tempfile::tempdir().unwrap();
    manifest(dir.path(), "");
    fs::write(dir.path().join("app.py"), "import requests\n").unwrap();
    let (code, result) = scan(dir.path(), &["--requirements", "missing.txt"]);
    assert_eq!(code, 1);
    assert!(result["scan_errors"][0]["file"]
        .as_str()
        .unwrap()
        .ends_with("missing.txt"));
    assert!(result["missing"].as_array().unwrap().is_empty());
    fs::write(
        dir.path().join("requirements.txt"),
        "requests\n-r nested.txt\n",
    )
    .unwrap();
    fs::write(dir.path().join("nested.txt"), "-r absent.txt\n").unwrap();
    let (code, result) = scan(dir.path(), &[]);
    assert_eq!(code, 1);
    let error = &result["scan_errors"][0];
    assert!(error["file"].as_str().unwrap().ends_with("absent.txt"));
    let context = error["error"].as_str().unwrap();
    assert!(context.contains("requirements.txt") && context.contains("nested.txt"));
    fs::write(dir.path().join("absent.txt"), "").unwrap();
    let (code, result) = scan(dir.path(), &[]);
    assert_eq!(code, 0);
    assert_eq!(result["scan_complete"], true);
}

#[test]
fn dependency_test_classification_ignores_ancestors_above_project_root() {
    let dir = tempfile::tempdir().unwrap();
    for parent in ["normal", "tests"] {
        let root = dir.path().join(parent).join("project");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir(root.join("tests")).unwrap();
        fs::write(
            root.join("pyproject.toml"),
            "[project]\nname='example'\nversion='0.1'\n[dependency-groups]\ndev=['pytest']\n",
        )
        .unwrap();
        fs::write(root.join("src/app.py"), "import pytest\n").unwrap();
        fs::write(root.join("tests/test_app.py"), "import pytest\n").unwrap();
        for selected in [&root, &root.join("src"), &root.join("src/app.py")] {
            let (code, result) = scan(selected, &["--fail-on-missing"]);
            assert_eq!(code, 1, "{selected:?}");
            assert_eq!(result["dev_in_production"].as_array().unwrap().len(), 1);
            let locations = result["dev_in_production"][0]["locations"]
                .as_array()
                .unwrap();
            assert!(locations
                .iter()
                .all(|location| location["file"].as_str().unwrap().ends_with("src/app.py")));
        }
        let (code, result) = scan(&root.join("tests"), &["--fail-on-missing"]);
        assert_eq!(code, 0);
        assert!(result["dev_in_production"].as_array().unwrap().is_empty());
    }
}

#[test]
fn main_dependency_gates_include_declaration_errors() {
    let dir = tempfile::tempdir().unwrap();
    manifest(dir.path(), "");
    fs::write(dir.path().join("app.py"), "print('ok')\n").unwrap();
    fs::write(dir.path().join("requirements.txt"), "-r missing.txt\n").unwrap();
    let mut output = Vec::new();
    let code = run_with_args_to(
        vec![
            dir.path().to_string_lossy().into_owned(),
            "--deps".into(),
            "--json".into(),
        ],
        &mut output,
    )
    .unwrap();
    assert_eq!(code, 1);
    let result: Value = serde_json::from_slice(&output).unwrap();
    assert!(!result["parse_errors"].as_array().unwrap().is_empty());
    assert_eq!(
        result["analysis_summary"]["parse_errors_count"],
        result["parse_errors"].as_array().unwrap().len()
    );
}
