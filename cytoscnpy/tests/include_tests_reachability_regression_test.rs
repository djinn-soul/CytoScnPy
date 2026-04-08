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
