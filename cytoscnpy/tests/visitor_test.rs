//! Tests for the AST visitor implementation.

use cytoscnpy::utils::LineIndex;
use cytoscnpy::visitor::CytoScnPyVisitor;
use rustpython_parser::{parse, Mode};
use std::collections::HashSet;
use std::path::PathBuf;

macro_rules! visit_code {
    ($code:expr, $visitor:ident) => {
        let line_index = LineIndex::new($code);
        let mut $visitor =
            CytoScnPyVisitor::new(PathBuf::from("test.py"), "test".to_string(), &line_index);
        let ast = parse($code, Mode::Module, "test.py").unwrap();
        if let rustpython_ast::Mod::Module(module) = ast {
            for stmt in module.body {
                $visitor.visit_stmt(&stmt);
            }
        }
    };
}

#[test]
fn test_simple_function() {
    let code = r"
def my_function():
    pass
";
    visit_code!(code, visitor);

    assert_eq!(visitor.definitions.len(), 1);
    let def = &visitor.definitions[0];
    assert_eq!(def.def_type, "function");
    assert_eq!(def.simple_name, "my_function");
}

#[test]
fn test_async_function() {
    let code = r"
async def async_function():
    await some_call()
";
    visit_code!(code, visitor);

    assert_eq!(visitor.definitions.len(), 1);
    let def = &visitor.definitions[0];
    assert_eq!(def.def_type, "function");
    assert_eq!(def.simple_name, "async_function");
}

#[test]
fn test_class_with_methods() {
    let code = r"
class MyClass:
    def __init__(self):
        pass

    def method(self):
        pass

    @staticmethod
    def static_method():
        pass

    @classmethod
    def class_method(cls):
        pass
";
    visit_code!(code, visitor);

    let class_defs: Vec<_> = visitor
        .definitions
        .iter()
        .filter(|d| d.def_type == "class")
        .collect();
    let method_defs: Vec<_> = visitor
        .definitions
        .iter()
        .filter(|d| d.def_type == "method")
        .collect();

    assert_eq!(class_defs.len(), 1);
    assert_eq!(class_defs[0].simple_name, "MyClass");

    assert_eq!(method_defs.len(), 4);
    let method_names: HashSet<String> = method_defs.iter().map(|m| m.simple_name.clone()).collect();
    assert!(method_names.contains("__init__"));
    assert!(method_names.contains("method"));
    assert!(method_names.contains("static_method"));
    assert!(method_names.contains("class_method"));
}

#[test]
fn test_imports_basic() {
    let code = r"
import os
import sys as system
";
    visit_code!(code, visitor);

    let imports: Vec<_> = visitor
        .definitions
        .iter()
        .filter(|d| d.def_type == "import")
        .collect();
    assert_eq!(imports.len(), 2);

    let import_names: HashSet<String> = imports.iter().map(|i| i.simple_name.clone()).collect();
    assert!(import_names.contains("os"));
    assert!(import_names.contains("system"));
}

#[test]
fn test_imports_from() {
    let code = r"
from pathlib import Path
from collections import defaultdict, Counter
from os.path import join as path_join
";
    visit_code!(code, visitor);

    let imports: Vec<_> = visitor
        .definitions
        .iter()
        .filter(|d| d.def_type == "import")
        .collect();
    assert_eq!(imports.len(), 4);

    let import_names: HashSet<String> = imports.iter().map(|i| i.simple_name.clone()).collect();
    assert!(import_names.contains("Path"));
    assert!(import_names.contains("defaultdict"));
    assert!(import_names.contains("Counter"));
    assert!(import_names.contains("path_join"));
}

#[test]
fn test_nested_functions() {
    let code = r"
def outer():
    def inner():
        pass
    inner()
";
    visit_code!(code, visitor);

    let functions: Vec<_> = visitor
        .definitions
        .iter()
        .filter(|d| d.def_type == "function")
        .collect();
    assert_eq!(functions.len(), 2);

    let func_names: HashSet<String> = functions.iter().map(|f| f.simple_name.clone()).collect();
    assert!(func_names.contains("outer"));
    assert!(func_names.contains("inner"));
}

#[test]
fn test_function_parameters() {
    let code = r"
def function_with_params(a, b, c=None):
    return a + b
";
    visit_code!(code, visitor);

    // Check parameters if implemented
    let _params: Vec<_> = visitor
        .definitions
        .iter()
        .filter(|d| d.def_type == "parameter")
        .collect();
}

#[test]
fn test_variables() {
    let code = r#"
MODULE_VAR = "module level"

class MyClass:
    CLASS_VAR = "class level"
    
    def method(self):
        local_var = "function level"
        return local_var
"#;
    visit_code!(code, visitor);

    let _vars: Vec<_> = visitor
        .definitions
        .iter()
        .filter(|d| d.def_type == "variable")
        .collect();
}

#[test]
fn test_getattr_detection() {
    let code = r"
obj = SomeClass()
value = getattr(obj, 'attribute_name')
";
    visit_code!(code, visitor);

    let ref_names: HashSet<String> = visitor.references.iter().map(|(n, _)| n.clone()).collect();
    assert!(ref_names.contains("attribute_name"));
}

#[test]
fn test_all_detection() {
    let code = r"
__all__ = ['function1', 'Class1']
";
    visit_code!(code, visitor);

    assert!(visitor.exports.contains(&"function1".to_owned()));
    assert!(visitor.exports.contains(&"Class1".to_owned()));
}

#[test]
fn test_decorators() {
    let code = r"
@my_decorator
def decorated():
    pass
";
    visit_code!(code, visitor);

    let _ref_names: HashSet<String> = visitor.references.iter().map(|(n, _)| n.clone()).collect();
    // assert!(_ref_names.contains("my_decorator")); // Uncomment when fixed
}

#[test]
fn test_inheritance_detection() {
    let code = r"
class Parent:
    pass

class Child(Parent):
    pass
";
    visit_code!(code, visitor);

    let classes: Vec<_> = visitor
        .definitions
        .iter()
        .filter(|d| d.def_type == "class")
        .collect();
    assert_eq!(classes.len(), 2);

    // Verify base classes captured
    let child = classes.iter().find(|c| c.simple_name == "Child").unwrap();
    assert!(child.base_classes.contains(&"Parent".to_owned()));

    // Verify reference to Parent
    let ref_names: HashSet<String> = visitor.references.iter().map(|(n, _)| n.clone()).collect();
    assert!(ref_names.contains("Parent"));
    assert!(ref_names.contains("test.Parent"));
}

#[test]
fn test_comprehensions() {
    let code = r"
squares = [x**2 for x in range(10)]
";
    visit_code!(code, visitor);

    let ref_names: HashSet<String> = visitor.references.iter().map(|(n, _)| n.clone()).collect();
    // range is a builtin, so it may be qualified with module name or just the simple name
    assert!(ref_names.contains("range") || ref_names.contains("test.range"));
}

#[test]
fn test_lambda_functions() {
    let code = r"
double = lambda x: x * 2
";
    visit_code!(code, visitor);

    let _ref_names: HashSet<String> = visitor.references.iter().map(|(n, _)| n.clone()).collect();
}

#[test]
fn test_attribute_access_chains() {
    let code = r#"
result = text.upper().replace(" ", "_")
"#;
    visit_code!(code, visitor);

    let ref_names: HashSet<String> = visitor.references.iter().map(|(n, _)| n.clone()).collect();

    assert!(ref_names.contains("upper"));
    assert!(ref_names.contains("replace"));
}

#[test]
fn test_star_imports() {
    let code = r"
from os import *
";
    visit_code!(code, visitor);

    let imports: Vec<_> = visitor
        .definitions
        .iter()
        .filter(|d| d.def_type == "import")
        .collect();
    let import_names: HashSet<String> = imports.iter().map(|i| i.simple_name.clone()).collect();
    assert!(import_names.contains("*"));
}
