//! Regression tests for unreachable nested function reporting.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-unreachable-nested-tmp");
    std::fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("unreachable_nested_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[test]
fn test_nested_local_function_not_promoted_to_unreachable_finding() {
    let dir = project_tempdir();
    let file_path = dir.path().join("nested.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r"
def outer():
    def inner():
        return 1
    return inner()
"
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let outer = result
        .unused_functions
        .iter()
        .find(|d| d.full_name == "nested.outer");
    assert!(
        outer.is_some(),
        "Outer function should still be reported as unused"
    );

    let inner = result
        .unused_functions
        .iter()
        .find(|d| d.full_name == "nested.outer.inner");
    assert!(
        inner.is_none(),
        "Nested local helper should not be escalated as unreachable finding"
    );
}
