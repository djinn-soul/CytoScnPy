//! Tests for pragma suppression in unused variable detection.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-pragma-tmp");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("pragma_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[test]
fn test_unused_variable_suppression() {
    let dir = project_tempdir();
    let file_path = dir.path().join("suppressed.py");
    let mut file = File::create(&file_path).unwrap();
    write!(
        file,
        r"
def example():
    # This variable is unused, but should be ignored due to pragma
    x = 10  # pragma: no cytoscnpy
    return 1

def unsuppressed():
    # This variable is unused and SHOULD be reported
    y = 20
    return 1
"
    )
    .unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(100).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_vars: Vec<String> = result
        .unused_variables
        .iter()
        .map(|v| v.simple_name.clone())
        .collect();

    assert!(
        !unused_vars.contains(&"x".to_owned()),
        "Variable 'x' should be suppressed by # pragma: no cytoscnpy"
    );

    assert!(
        unused_vars.contains(&"y".to_owned()),
        "Variable 'y' should be reported as unused"
    );
}

#[test]
fn test_suppression_case_insensitivity() {
    let dir = project_tempdir();
    let file_path = dir.path().join("case_test.py");
    let mut file = File::create(&file_path).unwrap();
    write!(
        file,
        r"
def example():
    x = 10  # PRAGMA: NO CYTOSCNPY
    return 1
"
    )
    .unwrap();

    let mut cytoscnpy = CytoScnPy::default();
    let result = cytoscnpy.analyze(dir.path());

    let unused_vars: Vec<String> = result
        .unused_variables
        .iter()
        .map(|v| v.simple_name.clone())
        .collect();

    assert!(
        !unused_vars.contains(&"x".to_owned()),
        "Pragma should work with different casing"
    );
}
