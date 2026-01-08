//! Tests for class method context analysis.

// Test-specific lint suppressions
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use cytoscnpy::utils::LineIndex;
use cytoscnpy::visitor::CytoScnPyVisitor;
use ruff_python_parser::{parse, Mode};
use std::path::PathBuf;

#[test]
fn test_class_method_context() {
    let source = r"
class MyClass:
    def my_method(self):
        pass

    def another_method(self):
        self.my_method()
";
    let line_index = LineIndex::new(source);
    let mut visitor = CytoScnPyVisitor::new(
        PathBuf::from("test.py"),
        "test_module".to_owned(),
        &line_index,
    );

    let ast = parse(source, Mode::Module.into()).unwrap();
    if let ruff_python_ast::Mod::Module(module) = ast.into_syntax() {
        for stmt in module.body {
            visitor.visit_stmt(&stmt);
        }
    }

    // Check definitions
    let defs: Vec<String> = visitor
        .definitions
        .iter()
        .map(|d| d.full_name.clone())
        .collect();
    println!("Definitions: {defs:?}");
    assert!(defs.contains(&"test_module.MyClass".to_owned()));
    assert!(defs.contains(&"test_module.MyClass.my_method".to_owned()));
    assert!(defs.contains(&"test_module.MyClass.another_method".to_owned()));

    // Check references
    let refs: Vec<String> = visitor.references.iter().map(|r| r.0.clone()).collect();
    println!("References: {refs:?}");
    assert!(refs.contains(&"test_module.MyClass.my_method".to_owned()));
}
