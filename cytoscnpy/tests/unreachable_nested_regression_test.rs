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

#[test]
fn test_method_nested_local_function_call_is_counted_as_usage() {
    let dir = project_tempdir();
    let file_path = dir.path().join("nested_in_method.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
class Processor:
    def transform(self, value):
        def normalize(item):
            return item.strip().lower()

        def unused_local(item):
            return item.upper()

        return normalize(value)

processor = Processor()
print(processor.transform(" Example "))
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let unused_function_names: Vec<_> = result
        .unused_functions
        .iter()
        .map(|d| d.full_name.as_str())
        .collect();
    let unused_method_names: Vec<_> = result
        .unused_methods
        .iter()
        .map(|d| d.full_name.as_str())
        .collect();
    assert!(
        !unused_function_names.contains(&"nested_in_method.Processor.transform.normalize"),
        "Called local helper inside method should not be reported unused; got: {unused_function_names:?}"
    );
    assert!(
        !unused_method_names.contains(&"nested_in_method.Processor.transform.normalize"),
        "Called local helper inside method should not be reported as an unused method; got: {unused_method_names:?}"
    );
    assert!(
        unused_function_names.contains(&"nested_in_method.Processor.transform.unused_local"),
        "Unused local helper inside method should still be reported; got: {unused_function_names:?}"
    );
}
