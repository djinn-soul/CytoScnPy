//! Tests for the AST visitor module.
#![allow(clippy::unwrap_used)]
#![allow(clippy::iter_kv_map)]

use cytoscnpy::utils::LineIndex;
use cytoscnpy::visitor::CytoScnPyVisitor;
use ruff_python_parser::{parse, Mode};
use std::collections::HashSet;
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

    let ref_names: HashSet<String> = visitor.references.iter().map(|(n, _)| n.clone()).collect();
    assert!(ref_names.contains("test.my_decorator"));
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

#[test]
fn test_try_except_handling() {
    let code = r"
def process_file():
    try:
        result = open_file()
    except FileNotFoundError:
        handle_missing()
    except ValueError as e:
        log_error(e)
    except (TypeError, KeyError):
        handle_type_error()
    finally:
        cleanup()
";
    visit_code!(code, visitor);

    // Check that function is defined
    let functions: Vec<_> = visitor
        .definitions
        .iter()
        .filter(|d| d.def_type == "function")
        .collect();
    assert_eq!(functions.len(), 1);
    assert_eq!(functions[0].simple_name, "process_file");

    // Check that references in try/except bodies are captured
    let ref_names: HashSet<String> = visitor.references.iter().map(|(n, _)| n.clone()).collect();

    // References from try body
    assert!(ref_names.contains("open_file") || ref_names.contains("test.open_file"));

    // References from except handlers
    assert!(ref_names.contains("handle_missing") || ref_names.contains("test.handle_missing"));
    assert!(ref_names.contains("log_error") || ref_names.contains("test.log_error"));
    assert!(
        ref_names.contains("handle_type_error") || ref_names.contains("test.handle_type_error")
    );

    // References from finally block
    assert!(ref_names.contains("cleanup") || ref_names.contains("test.cleanup"));

    // Exception type references
    assert!(
        ref_names.contains("FileNotFoundError") || ref_names.contains("test.FileNotFoundError")
    );
    assert!(ref_names.contains("ValueError") || ref_names.contains("test.ValueError"));
    assert!(ref_names.contains("TypeError") || ref_names.contains("test.TypeError"));
    assert!(ref_names.contains("KeyError") || ref_names.contains("test.KeyError"));
}

#[test]
fn test_future_imports_ignored() {
    // __future__ imports are compiler directives, not real imports
    // They should NOT be added as definitions to avoid false "unused import" positives
    let code = r"
from __future__ import annotations
from __future__ import division, print_function

import os
from pathlib import Path
";
    visit_code!(code, visitor);

    // Only real imports should be added as definitions
    let import_defs: Vec<_> = visitor
        .definitions
        .iter()
        .filter(|d| d.def_type == "import")
        .collect();

    let import_names: HashSet<String> = import_defs.iter().map(|d| d.simple_name.clone()).collect();

    // __future__ imports should NOT be in definitions
    assert!(
        !import_names.contains("annotations"),
        "annotations from __future__ should not be tracked as import"
    );
    assert!(
        !import_names.contains("division"),
        "division from __future__ should not be tracked as import"
    );
    assert!(
        !import_names.contains("print_function"),
        "print_function from __future__ should not be tracked as import"
    );

    // Regular imports SHOULD be tracked
    assert!(
        import_names.contains("os"),
        "regular import 'os' should be tracked"
    );
    assert!(
        import_names.contains("Path"),
        "regular import 'Path' should be tracked"
    );

    // Total should be 2 (os, Path) - not 5
    assert_eq!(
        import_defs.len(),
        2,
        "Only 2 real imports should be tracked, not __future__ imports"
    );
}

#[test]
fn test_alias_resolution() {
    // When using an aliased import, both the alias and original name should be tracked
    let code = r"
import pandas as pd
from os.path import join as path_join

df = pd.DataFrame()
result = path_join('a', 'b')
";
    visit_code!(code, visitor);

    let ref_names: HashSet<String> = visitor.references.iter().map(|(n, _)| n.clone()).collect();

    // Using 'pd' should add reference to 'pandas' (the original)
    assert!(
        ref_names.contains("pandas"),
        "Using alias 'pd' should resolve to original 'pandas'"
    );

    // Using 'path_join' should add reference to 'os.path.join' (the original qualified name)
    assert!(
        ref_names.contains("os.path.join"),
        "Using alias 'path_join' should resolve to original 'os.path.join'"
    );

    // Should also add simple name 'join' for qualified imports
    assert!(
        ref_names.contains("join"),
        "Using qualified alias should also add simple name 'join'"
    );
}
