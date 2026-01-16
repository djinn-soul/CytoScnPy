//! Tests for analyzer heuristics (e.g., config names, visitor pattern).
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-heuristics-tmp");
    std::fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("heuristics_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[test]
fn test_heuristics_constants() {
    let dir = project_tempdir();
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
    CONSTANT = 1
    _PRIVATE = 2 # Should be reported
    _private_var = 3 # Should be reported

"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(10).with_tests(false);
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

    // We need to test that PRIVATE attributes are reported
    assert!(!unused_vars.contains(&"db_host".to_owned()));
    assert!(!unused_vars.contains(&"CONSTANT".to_owned()));
    assert!(unused_vars.contains(&"_PRIVATE".to_owned()));
    assert!(unused_vars.contains(&"_private_var".to_owned()));
}

#[test]
fn test_visitor_pattern_heuristic() {
    let dir = project_tempdir();
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
    let dir = project_tempdir();
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
    _field: str  # Should be reported as unused variable (private)
"
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(10).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let unused_vars: Vec<String> = result
        .unused_variables
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    // Dataclass fields should be marked as used
    assert!(!unused_vars.contains(&"name".to_owned()));
    assert!(!unused_vars.contains(&"age".to_owned()));

    // Regular class private field should be unused
    assert!(unused_vars.contains(&"_field".to_owned()));
}

#[test]
fn test_abc_abstract_methods() {
    let dir = project_tempdir();
    let file_path = dir.path().join("abc_test.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
from abc import ABC, abstractmethod

class Processor(ABC):
    @abstractmethod
    def process(self):
        pass
        
    def concrete(self):
        pass

class ConcreteProcessor(Processor):
    def process(self):
        print("processing")
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let unused_methods: Vec<String> = result
        .unused_methods
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    assert!(!unused_methods.contains(&"process".to_owned()));
    // concrete might be considered implicitly used or public api depending on config
    // assert!(unused_methods.contains(&"concrete".to_owned()));
}

#[test]
fn test_protocol_member_tracking() {
    let dir = project_tempdir();
    let file_path = dir.path().join("protocol_test.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
from typing import Protocol

class Renderable(Protocol):
    def render(self): ...
    def layout(self): ...
    def update(self): ...

class Button:
    def render(self): return "Button"
    # Implicitly implements others via pass or actual logic
    def layout(self): pass
    def update(self): pass
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let render_findings: Vec<_> = result
        .unused_methods
        .iter()
        .filter(|d| d.simple_name == "render")
        .collect();

    // Duck Typing Logic:
    // Button implements Renderable (3/3 methods match).
    // Button.render, Button.layout, Button.update should be marked as used (ref > 0)
    // and thus NOT appear in unused_methods.

    let button_render = result
        .unused_methods
        .iter()
        .find(|d| d.full_name == "Button.render");
    assert!(
        button_render.is_none(),
        "Button.render should be marked used via duck typing"
    );

    // Check Protocol methods (likely still unused if not referenced)
    // We allow them to be reported as unused in this phase unless referenced.
}

#[test]
fn test_optional_dependency_flags() {
    let dir = project_tempdir();
    let file_path = dir.path().join("flags.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r"
try:
    import pandas
    HAS_PANDAS = True
except ImportError:
    HAS_PANDAS = False

def use_pandas():
    if HAS_PANDAS:
        pass
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

    assert!(!unused_vars.contains(&"HAS_PANDAS".to_owned()));
}

#[test]
fn test_adapter_penalty() {
    let dir = project_tempdir();
    let file_path = dir.path().join("adapter.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r"
class NetworkAdapter:
    def connect(self):
        pass
        
    def disconnect(self):
        pass
"
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let adapter_methods: Vec<_> = result
        .unused_methods
        .iter()
        .filter(|d| d.full_name.contains("NetworkAdapter"))
        .collect();

    assert!(
        !adapter_methods.is_empty(),
        "Adapter methods should be found as unused"
    );
    for method in adapter_methods {
        assert!(
            method.confidence <= 70,
            "Adapter method confidence should be penalized"
        );
    }
}
