//! Regression tests for walrus-expression reference tracking.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-walrus-reference-tmp");
    std::fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("walrus_reference_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[test]
fn test_walrus_reads_are_counted_for_unused_variables() {
    let dir = project_tempdir();
    let file_path = dir.path().join("walrus_case.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
from collections.abc import Callable
from typing import Any

def _check(name: str, line_content: str) -> str | None:
    return None

def verify_item(item_type: str) -> str | None:
    checkers: dict[str, Callable[[str, str], str | None]] = {{
        "variable": _check,
    }}
    if checker := checkers.get(item_type):
        return checker("x", "y")
    return None
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(10).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let checkers_var = result
        .unused_variables
        .iter()
        .find(|d| d.full_name == "walrus_case.verify_item.checkers");
    assert!(
        checkers_var.is_none(),
        "Variables read through walrus expressions should not be reported unused"
    );

    let nested_callable = result
        .unused_functions
        .iter()
        .find(|d| d.simple_name == "_check");
    assert!(
        nested_callable.is_none_or(|def| !def.is_unreachable),
        "Captured callables referenced through dispatcher maps should not be marked unreachable"
    );
}
