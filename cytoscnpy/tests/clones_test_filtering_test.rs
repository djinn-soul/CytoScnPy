//! Regression tests for clone detection test-file filtering.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use cytoscnpy::commands::{generate_clone_findings_with_thresholds, run_clones, CloneOptions};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

const CLONE_SOURCE: &str = "
def duplicate(x):
    a = 1
    b = 2
    c = a + b
    return x
";

fn create_project(files: &[(&str, &str)]) -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    for (relative_path, source) in files {
        let path = directory.path().join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, source).unwrap();
    }
    directory
}

fn run_clone_json(path: PathBuf, include_tests: bool) -> serde_json::Value {
    let options = CloneOptions {
        similarity: 0.8,
        json: true,
        include_tests,
        with_cst: true,
        ..CloneOptions::default()
    };
    let mut output = Vec::new();

    run_clones(&[path], &options, &mut output).expect("clone detection should succeed");

    serde_json::from_slice(&output).expect("clone detection should emit JSON")
}

#[test]
fn clone_command_excludes_tests_by_default() {
    let production_source = "
def production_only(value):
    first = value + 1
    second = first * 2
    third = second - 3
    return third
";
    let project = create_project(&[
        ("src/main.py", production_source),
        ("tests/test_one.py", CLONE_SOURCE),
        ("tests/test_two.py", CLONE_SOURCE),
    ]);

    let findings = run_clone_json(project.path().to_path_buf(), false);

    assert!(findings.as_array().unwrap().is_empty());
}

#[test]
fn clone_command_includes_tests_when_requested() {
    let project = create_project(&[
        ("tests/test_one.py", CLONE_SOURCE),
        ("tests/test_two.py", CLONE_SOURCE),
    ]);

    let findings = run_clone_json(project.path().to_path_buf(), true);

    assert!(!findings.as_array().unwrap().is_empty());
}

#[test]
fn clone_command_emits_empty_json_when_only_tests_are_filtered() {
    let project = create_project(&[
        ("tests/test_one.py", CLONE_SOURCE),
        ("tests/test_two.py", CLONE_SOURCE),
    ]);

    let findings = run_clone_json(project.path().to_path_buf(), false);

    assert!(findings.as_array().unwrap().is_empty());
}

#[test]
fn clone_command_honors_force_included_folders() {
    let project = create_project(&[
        (".venv/one.py", CLONE_SOURCE),
        (".venv/two.py", CLONE_SOURCE),
    ]);
    let options = CloneOptions {
        similarity: 0.8,
        json: true,
        include_folders: vec![".venv".to_owned()],
        with_cst: true,
        ..CloneOptions::default()
    };
    let mut output = Vec::new();

    run_clones(&[project.path().to_path_buf()], &options, &mut output).unwrap();
    let findings: serde_json::Value = serde_json::from_slice(&output).unwrap();

    assert!(!findings.as_array().unwrap().is_empty());
}

#[test]
fn clone_test_context_lowers_confidence() {
    use cytoscnpy::clones::{CloneInstance, ClonePair, CloneType, NodeKind};

    let instance = |file: &str| CloneInstance {
        file: PathBuf::from(file),
        start_line: 1,
        end_line: 5,
        start_byte: 0,
        end_byte: 50,
        normalized_hash: 1,
        name: Some("duplicate".to_owned()),
        node_kind: NodeKind::Function,
    };
    let pair = |first: &str, second: &str| ClonePair {
        instance_a: instance(first),
        instance_b: instance(second),
        similarity: 0.95,
        clone_type: CloneType::Type2,
        edit_distance: 3,
    };

    let production =
        generate_clone_findings_with_thresholds(&[pair("one.py", "two.py")], &[], false, 100, 90);
    let tests = generate_clone_findings_with_thresholds(
        &[pair("tests/test_one.py", "tests/test_two.py")],
        &[],
        false,
        100,
        90,
    );

    assert_eq!(production.len(), 2);
    assert!(tests.is_empty());
}

#[test]
fn clone_suggestion_threshold_can_suppress_findings() {
    let files = vec![
        (PathBuf::from("one.py"), CLONE_SOURCE.to_owned()),
        (PathBuf::from("two.py"), CLONE_SOURCE.to_owned()),
    ];
    let detector = cytoscnpy::clones::CloneDetector::new();
    let result = detector.detect(&files);

    let findings = generate_clone_findings_with_thresholds(&result.pairs, &files, false, 101, 101);

    assert!(findings.is_empty());
}

#[test]
fn path_and_in_memory_clone_apis_classify_identically() {
    let first = "
def calculate_total(items):
    total = 0
    for item in items:
        total += item
    return total
";
    let second = "
def sum_values(values):
    result = 0
    for value in values:
        result += value
    return result
";
    let project = create_project(&[("one.py", first), ("two.py", second)]);
    let paths = vec![project.path().join("one.py"), project.path().join("two.py")];
    let files = vec![
        (paths[0].clone(), first.to_owned()),
        (paths[1].clone(), second.to_owned()),
    ];
    let detector = cytoscnpy::clones::CloneDetector::new();

    let from_paths = detector.detect_from_paths(&paths);
    let from_memory = detector.detect(&files);

    assert_eq!(
        from_paths.pairs[0].clone_type,
        from_memory.pairs[0].clone_type
    );
}
