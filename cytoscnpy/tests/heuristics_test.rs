//! Tests for analyzer heuristics (e.g., config names, visitor pattern).
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_heuristics_constants() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("settings.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
class AppSettings:
    DEBUG = True
    SECRET_KEY = "secret"
    db_host = "localhost"  # Lowercase should NOT be ignored

class Config:
    API_KEY = "123"

class OtherClass:
    CONSTANT = 1  # Should NOT be ignored (wrong class name)
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    // DEBUG and SECRET_KEY and API_KEY should be ignored (confidence 0)
    // db_host and CONSTANT should be reported

    let unused_vars: Vec<String> = result
        .unused_variables
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    assert!(!unused_vars.contains(&"DEBUG".to_owned()));
    assert!(!unused_vars.contains(&"SECRET_KEY".to_owned()));
    assert!(!unused_vars.contains(&"API_KEY".to_owned()));

    assert!(unused_vars.contains(&"db_host".to_owned()));
    assert!(unused_vars.contains(&"CONSTANT".to_owned()));
}

#[test]
fn test_visitor_pattern_heuristic() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("visitor.py");
    let mut file = File::create(&file_path).unwrap();

    // Visitor class with visit_ methods
    writeln!(
        file,
        r"
class MyVisitor:
    def visit_node(self):
        pass

    def leave_node(self):
        pass

    def transform_node(self):
        pass

    def other_method(self):
        pass
"
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let unused_method_names: Vec<String> = result
        .unused_methods
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    assert!(!unused_method_names.contains(&"visit_node".to_owned()));
    assert!(!unused_method_names.contains(&"leave_node".to_owned()));
    assert!(!unused_method_names.contains(&"transform_node".to_owned()));

    assert!(unused_method_names.contains(&"other_method".to_owned()));
}

#[test]
fn test_dataclass_fields() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("models.py");
    let mut file = File::create(&file_path).unwrap();

    // Dataclass with fields
    writeln!(
        file,
        r"
from dataclasses import dataclass

@dataclass
class User:
    name: str
    age: int = 0

class RegularClass:
    field: str  # Should be reported as unused variable
"
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let unused_vars: Vec<String> = result
        .unused_variables
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    // Dataclass fields should be marked as used
    assert!(!unused_vars.contains(&"name".to_owned()));
    assert!(!unused_vars.contains(&"age".to_owned()));

    // Regular class field should be unused
    assert!(unused_vars.contains(&"field".to_owned()));
}
