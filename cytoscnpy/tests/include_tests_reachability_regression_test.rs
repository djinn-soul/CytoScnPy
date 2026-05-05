//! Regression tests for test-root reachability behavior.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::config::ProjectType;
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-include-tests-roots-tmp");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("include_tests_roots_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[test]
fn source_symbols_referenced_only_by_tests_are_reachable_when_include_tests_enabled() {
    let dir = project_tempdir();
    let pkg_dir = dir.path().join("pkg");
    let tests_dir = dir.path().join("tests");
    fs::create_dir_all(&pkg_dir).unwrap();
    fs::create_dir_all(&tests_dir).unwrap();

    let mut init_file = File::create(pkg_dir.join("__init__.py")).unwrap();
    writeln!(init_file, "from .service import use_from_tests").unwrap();

    let mut source_file = File::create(pkg_dir.join("service.py")).unwrap();
    writeln!(
        source_file,
        r"
def use_from_tests() -> int:
    return 7
"
    )
    .unwrap();

    let mut test_file = File::create(tests_dir.join("test_service.py")).unwrap();
    writeln!(
        test_file,
        r"
from pkg.service import use_from_tests

def test_use_from_tests() -> None:
    assert use_from_tests() == 7
"
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(0).with_tests(true);
    analyzer.config.cytoscnpy.project_type = Some(ProjectType::Application);
    let result = analyzer.analyze(dir.path());

    let unused_source_function = result
        .unused_functions
        .iter()
        .find(|d| d.full_name == "pkg.service.use_from_tests");
    assert!(
        unused_source_function.is_none(),
        "Functions referenced only by tests should not be unreachable when include_tests=true"
    );
}

#[test]
fn prod_function_only_called_from_tests_is_flagged_as_unused() {
    // Regression: before the fix, test-file references leaked into ref_counts and
    // masked prod functions that had zero non-test callers.
    let dir = project_tempdir();
    let src_dir = dir.path().join("src");
    let tests_dir = dir.path().join("tests");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&tests_dir).unwrap();

    // Production module — no __init__.py re-export, no prod caller
    let mut source_file = File::create(src_dir.join("helpers.py")).unwrap();
    writeln!(
        source_file,
        r"
def internal_helper() -> int:
    return 42
"
    )
    .unwrap();

    // Test file is the only caller
    let mut test_file = File::create(tests_dir.join("test_helpers.py")).unwrap();
    writeln!(
        test_file,
        r"
from src.helpers import internal_helper

def test_helper() -> None:
    assert internal_helper() == 42
"
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(0).with_tests(true);
    analyzer.config.cytoscnpy.project_type = Some(ProjectType::Application);
    let result = analyzer.analyze(dir.path());

    let unused = result
        .unused_functions
        .iter()
        .find(|d| d.full_name == "src.helpers.internal_helper");
    assert!(
        unused.is_some(),
        "Prod function with no non-test callers should be flagged unused even when include_tests=true"
    );
}
