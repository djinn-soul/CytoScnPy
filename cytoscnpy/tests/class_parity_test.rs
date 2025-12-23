//! Tests for class-related parity with Python.
#![allow(clippy::unwrap_used)]

use cytoscnpy::utils::LineIndex;
use cytoscnpy::visitor::CytoScnPyVisitor;
use ruff_python_parser::{parse, Mode};
use std::path::PathBuf;

#[test]
fn test_class_parity_features() {
    let source = r"
class BaseClass:
    pass

class ChildClass(BaseClass):
    def instance_method(self):
        self.helper()

    def class_method(cls):
        cls.static_helper()
        
    def helper(self):
        pass
        
    def static_helper(cls):
        pass
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

    let refs: Vec<String> = visitor.references.iter().map(|r| r.0.clone()).collect();
    println!("References: {refs:?}");

    // 1. Verify Base Class reference
    assert!(
        refs.contains(&"BaseClass".to_owned()),
        "BaseClass should be referenced"
    );

    // 2. Verify self.method() -> ChildClass.helper
    assert!(
        refs.contains(&"test_module.ChildClass.helper".to_owned()),
        "self.helper() should resolve to ChildClass.helper"
    );

    // 3. Verify cls.method() -> ChildClass.static_helper
    assert!(
        refs.contains(&"test_module.ChildClass.static_helper".to_owned()),
        "cls.static_helper() should resolve to ChildClass.static_helper"
    );
}
