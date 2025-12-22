//! Tests for the message field generation in Definition struct.
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::doc_markdown)]

use cytoscnpy::utils::LineIndex;
use cytoscnpy::visitor::CytoScnPyVisitor;
use ruff_python_parser::{parse, Mode};
use std::path::PathBuf;

macro_rules! visit_code {
    ($code:expr, $visitor:ident) => {
        let line_index = LineIndex::new($code);
        let mut $visitor =
            CytoScnPyVisitor::new(PathBuf::from("test.py"), "test".to_string(), &line_index);
        let ast = parse($code, Mode::Module.into()).unwrap();
        if let ruff_python_ast::Mod::Module(module) = ast.into_syntax() {
            for stmt in module.body {
                $visitor.visit_stmt(&stmt);
            }
        }
    };
}

/// Test that message field is correctly generated for different def_types.
#[test]
fn test_message_field_for_function() {
    let code = r"
def unused_function():
    pass
";
    visit_code!(code, visitor);

    let func = visitor
        .definitions
        .iter()
        .find(|d| d.def_type == "function")
        .expect("Should have a function definition");

    assert!(
        func.message.is_some(),
        "Function should have a message field"
    );
    let msg = func.message.as_ref().unwrap();
    assert!(
        msg.contains("is defined but never used"),
        "Function message format is wrong: {}",
        msg
    );
    assert!(
        msg.contains("unused_function"),
        "Function message should contain function name: {}",
        msg
    );
}

/// Test that message field is correctly generated for classes.
#[test]
fn test_message_field_for_class() {
    let code = r"
class UnusedClass:
    pass
";
    visit_code!(code, visitor);

    let cls = visitor
        .definitions
        .iter()
        .find(|d| d.def_type == "class")
        .expect("Should have a class definition");

    assert!(cls.message.is_some(), "Class should have a message field");
    let msg = cls.message.as_ref().unwrap();
    assert!(
        msg.contains("Class"),
        "Class message should start with 'Class': {}",
        msg
    );
    assert!(
        msg.contains("is defined but never used"),
        "Class message format is wrong: {}",
        msg
    );
    assert!(
        msg.contains("UnusedClass"),
        "Class message should contain class name: {}",
        msg
    );
}

/// Test that message field is correctly generated for methods.
#[test]
fn test_message_field_for_method() {
    let code = r"
class MyClass:
    def my_method(self):
        pass
";
    visit_code!(code, visitor);

    let method = visitor
        .definitions
        .iter()
        .find(|d| d.def_type == "method")
        .expect("Should have a method definition");

    assert!(
        method.message.is_some(),
        "Method should have a message field"
    );
    let msg = method.message.as_ref().unwrap();
    assert!(
        msg.contains("Method"),
        "Method message should start with 'Method': {}",
        msg
    );
    assert!(
        msg.contains("is defined but never used"),
        "Method message format is wrong: {}",
        msg
    );
}

/// Test that message uses simple_name, not full qualified name.
#[test]
fn test_message_uses_simple_name() {
    let code = r"
class MyClass:
    def my_method(self):
        pass
";
    visit_code!(code, visitor);

    let method = visitor
        .definitions
        .iter()
        .find(|d| d.def_type == "method")
        .expect("Should have a method definition");

    let msg = method.message.as_ref().unwrap();
    // Should contain 'my_method' not 'MyClass.my_method'
    assert!(
        msg.contains("'my_method'"),
        "Message should use simple name in quotes: {}",
        msg
    );
    assert!(
        !msg.contains("MyClass.my_method"),
        "Message should NOT contain full qualified name: {}",
        msg
    );
}

/// Test import message format.
#[test]
fn test_message_field_for_import() {
    let code = r"
import os
";
    visit_code!(code, visitor);

    let import = visitor
        .definitions
        .iter()
        .find(|d| d.def_type == "import")
        .expect("Should have an import definition");

    assert!(
        import.message.is_some(),
        "Import should have a message field"
    );
    let msg = import.message.as_ref().unwrap();
    assert!(
        msg.contains("is imported but never used"),
        "Import message format is wrong: {}",
        msg
    );
}

/// Test variable message format.
#[test]
fn test_message_field_for_variable() {
    let code = r"
unused_variable = 42
";
    visit_code!(code, visitor);

    let var = visitor
        .definitions
        .iter()
        .find(|d| d.def_type == "variable")
        .expect("Should have a variable definition");

    assert!(
        var.message.is_some(),
        "Variable should have a message field"
    );
    let msg = var.message.as_ref().unwrap();
    assert!(
        msg.contains("Variable"),
        "Variable message should start with 'Variable': {}",
        msg
    );
    assert!(
        msg.contains("is assigned but never used"),
        "Variable message format is wrong: {}",
        msg
    );
}
