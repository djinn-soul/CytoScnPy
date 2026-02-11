//! Regression test for pytest fixture-name false negatives in dead-code detection.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-pytest-fixture-regression");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("pytest_fixture_regression_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[test]
fn non_test_function_named_like_pytest_fixture_is_still_flagged_unused() {
    let dir = project_tempdir();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();
    let content = r#"
def client():
    return "unused"
"#;
    write!(file, "{content}").unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(1).with_tests(false);
    let result = analyzer.analyze(dir.path());

    assert!(
        result
            .unused_functions
            .iter()
            .any(|item| item.simple_name == "client"),
        "Expected unused production function `client` to be reported"
    );
}
