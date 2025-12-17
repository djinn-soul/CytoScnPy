//! Tests for unused parameter detection.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_simple_unused_parameter() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r"
def function_with_unused(a, b, c):
    return b + c  # 'a' is unused
";
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_params: Vec<String> = result
        .unused_parameters
        .iter()
        .map(|p| p.simple_name.clone())
        .collect();

    assert!(
        unused_params.contains(&"a".to_owned()),
        "Parameter 'a' should be detected as unused"
    );
}

#[test]
fn test_all_parameters_used() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r"
def all_used(a, b, c):
    return a + b + c
";
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    assert!(
        result.unused_parameters.is_empty(),
        "No parameters should be reported as unused"
    );
}

#[test]
fn test_self_not_reported() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r"
class MyClass:
    def method(self, param):
        return 42  # param unused, but self never reported
";
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_params: Vec<String> = result
        .unused_parameters
        .iter()
        .map(|p| p.simple_name.clone())
        .collect();

    assert!(
        !unused_params.contains(&"self".to_owned()),
        "self should never be reported as unused"
    );
    assert!(
        unused_params.contains(&"param".to_owned()),
        "unused param should be detected"
    );
}

#[test]
fn test_cls_not_reported() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r"
class MyClass:
    @classmethod
    def class_method(cls, param):
        return 42  # param unused, but cls never reported
";
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_params: Vec<String> = result
        .unused_parameters
        .iter()
        .map(|p| p.simple_name.clone())
        .collect();

    assert!(
        !unused_params.contains(&"cls".to_owned()),
        "cls should never be reported as unused"
    );
}

#[test]
fn test_args_kwargs() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r"
def func_with_varargs(a, *args, **kwargs):
    return a  # args and kwargs unused
";
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_params: Vec<String> = result
        .unused_parameters
        .iter()
        .map(|p| p.simple_name.clone())
        .collect();

    // args and kwargs might be required for interface compliance
    // They should be detected but with lower confidence
    assert!(
        unused_params.contains(&"args".to_owned()) || unused_params.contains(&"kwargs".to_owned()),
        "*args or **kwargs should be detected"
    );
}

#[test]
fn test_keyword_only_parameters() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r"
def func_with_kwonly(a, *, kwonly_param):
    return a  # kwonly_param unused
";
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_params: Vec<String> = result
        .unused_parameters
        .iter()
        .map(|p| p.simple_name.clone())
        .collect();

    assert!(
        unused_params.contains(&"kwonly_param".to_owned()),
        "Keyword-only parameter should be detected"
    );
}

#[test]
fn test_default_values() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r"
def func_with_defaults(a, b=10, c=20):
    return b  # a and c unused
";
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_params: Vec<String> = result
        .unused_parameters
        .iter()
        .map(|p| p.simple_name.clone())
        .collect();

    assert!(
        unused_params.contains(&"a".to_owned()),
        "Parameter 'a' should be detected"
    );
    assert!(
        unused_params.contains(&"c".to_owned()),
        "Parameter 'c' should be detected"
    );
}

#[test]
fn test_nested_functions() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r"
def outer(x, y):
    def inner(a, b):
        return a  # b unused in inner
    return inner(x, y)  # x used, y used
";
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_params: Vec<String> = result
        .unused_parameters
        .iter()
        .map(|p| p.simple_name.clone())
        .collect();

    // inner.b should be unused
    assert!(
        unused_params.contains(&"b".to_owned()),
        "Parameter 'b' in inner function should be detected"
    );
}

#[test]
fn test_type_annotated_params() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r"
def typed_func(a: int, b: str, c: float) -> int:
    return a  # b and c unused even with type hints
";
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_params: Vec<String> = result
        .unused_parameters
        .iter()
        .map(|p| p.simple_name.clone())
        .collect();

    assert!(
        unused_params.contains(&"b".to_owned()),
        "Type-annotated param 'b' should still be detected"
    );
    assert!(
        unused_params.contains(&"c".to_owned()),
        "Type-annotated param 'c' should still be detected"
    );
}

#[test]
fn test_param_in_comprehension() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r"
def with_comprehension(items, multiplier):
    return [x * multiplier for x in items]
";
    write!(file, "{content}").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    // Both params should be detected as used
    assert!(
        result.unused_parameters.is_empty(),
        "Parameters used in comprehension should not be detected as unused"
    );
}

#[test]
fn test_confidence_threshold_70() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r"
def func(unused_param):
    return 42
";
    write!(file, "{content}").unwrap();

    // Test with confidence threshold 60 (should detect)
    let mut cytoscnpy_60 = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result_60 = cytoscnpy_60.analyze(dir.path());
    assert!(
        !result_60.unused_parameters.is_empty(),
        "Should detect with threshold 60"
    );

    // Test with confidence threshold 80 (should still detect since params have confidence 100)
    let mut cytoscnpy_80 = CytoScnPy::default().with_confidence(80).with_tests(false);
    let result_80 = cytoscnpy_80.analyze(dir.path());
    assert!(
        !result_80.unused_parameters.is_empty(),
        "Should still detect with threshold 80 (params have confidence 100)"
    );
}
