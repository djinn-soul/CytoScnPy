//! Tests for local scope analysis.
#![allow(clippy::case_sensitive_file_extension_comparisons)]

use cytoscnpy::utils::LineIndex;
use cytoscnpy::visitor::CytoScnPyVisitor;
use ruff_python_parser::{parse, Mode};
use std::path::PathBuf;

macro_rules! visit_code {
    ($code:expr, $visitor:ident) => {
        let line_index = LineIndex::new($code);
        let mut $visitor = CytoScnPyVisitor::new(
            PathBuf::from("test.py"),
            "test_module".to_string(),
            &line_index,
        );
        let ast = parse($code, Mode::Module.into()).unwrap();
        if let ruff_python_ast::Mod::Module(module) = ast.into_syntax() {
            for stmt in module.body {
                $visitor.visit_stmt(&stmt);
            }
        }
    };
}

#[test]
fn test_nested_functions_different_vars() {
    let code = r"
def outer():
    x = 1
    def inner():
        y = 2
        return y
    return x
";
    visit_code!(code, visitor);

    let def_names: Vec<String> = visitor
        .definitions
        .iter()
        .map(|d| d.full_name.clone())
        .collect();
    assert!(
        def_names.contains(&"test_module.outer".to_owned()),
        "Missing outer function"
    );
    assert!(
        def_names.contains(&"test_module.outer.x".to_owned()),
        "Missing outer.x variable"
    );
    assert!(
        def_names.contains(&"test_module.outer.inner".to_owned()),
        "Missing inner function"
    );
    assert!(
        def_names.contains(&"test_module.outer.inner.y".to_owned()),
        "Missing inner.y variable"
    );
}

#[test]
fn test_nested_functions_same_var_name() {
    let code = r"
def outer():
    x = 1
    def inner():
        x = 2
        return x
    return x
";
    visit_code!(code, visitor);

    let def_names: Vec<String> = visitor
        .definitions
        .iter()
        .map(|d| d.full_name.clone())
        .collect();
    assert!(
        def_names.contains(&"test_module.outer.x".to_owned()),
        "Missing outer.x"
    );
    assert!(
        def_names.contains(&"test_module.outer.inner.x".to_owned()),
        "Missing inner.x"
    );

    let x_defs: Vec<_> = def_names.iter().filter(|n| n.ends_with(".x")).collect();
    assert_eq!(
        x_defs.len(),
        2,
        "Expected exactly 2 x definitions, got: {x_defs:?}"
    );
}

#[test]
fn test_triple_nested_functions() {
    let code = r"
def level1():
    a = 1
    def level2():
        b = 2
        def level3():
            c = 3
            return c
        return b
    return a
";
    visit_code!(code, visitor);

    let def_names: Vec<String> = visitor
        .definitions
        .iter()
        .map(|d| d.full_name.clone())
        .collect();
    assert!(
        def_names.contains(&"test_module.level1.a".to_owned()),
        "Missing level1.a"
    );
    assert!(
        def_names.contains(&"test_module.level1.level2.b".to_owned()),
        "Missing level2.b"
    );
    assert!(
        def_names.contains(&"test_module.level1.level2.level3.c".to_owned()),
        "Missing level3.c"
    );
}

#[test]
fn test_sibling_functions_same_var() {
    let code = r"
def func_a():
    x = 1
    return x

def func_b():
    x = 2
    return x
";
    visit_code!(code, visitor);

    let def_names: Vec<String> = visitor
        .definitions
        .iter()
        .map(|d| d.full_name.clone())
        .collect();
    assert!(
        def_names.contains(&"test_module.func_a.x".to_owned()),
        "Missing func_a.x"
    );
    assert!(
        def_names.contains(&"test_module.func_b.x".to_owned()),
        "Missing func_b.x"
    );

    let x_defs: Vec<_> = def_names.iter().filter(|n| n.ends_with(".x")).collect();
    assert_eq!(
        x_defs.len(),
        2,
        "Expected 2 x definitions in sibling functions"
    );
}

#[test]
fn test_class_method_vs_local_var() {
    let code = r"
class MyClass:
    x = 1
    
    def method(self):
        y = 2
        return y
";
    visit_code!(code, visitor);

    let def_names: Vec<String> = visitor
        .definitions
        .iter()
        .map(|d| d.full_name.clone())
        .collect();
    assert!(
        def_names.contains(&"test_module.MyClass.x".to_owned()),
        "Missing class variable x"
    );
    assert!(
        def_names.contains(&"test_module.MyClass.method.y".to_owned()),
        "Missing method local var y"
    );
}

#[test]
fn test_function_parameters_are_scoped() {
    let code = r"
def outer(a):
    def inner(b):
        c = a + b
        return c
    return inner
";
    visit_code!(code, visitor);

    let def_names: Vec<String> = visitor
        .definitions
        .iter()
        .map(|d| d.full_name.clone())
        .collect();
    assert!(
        def_names.contains(&"test_module.outer.a".to_owned()),
        "Missing parameter outer.a"
    );
    assert!(
        def_names.contains(&"test_module.outer.inner.b".to_owned()),
        "Missing parameter inner.b"
    );
    assert!(
        def_names.contains(&"test_module.outer.inner.c".to_owned()),
        "Missing local var inner.c"
    );
}

#[test]
fn test_complex_nested_scenario() {
    let code = r#"
x = "module_level"

class OuterClass:
    x = "class_level"
    
    def outer_method(self):
        x = "method_level"
        
        def nested_func():
            x = "nested_func_level"
            return x
        
        return x
"#;
    visit_code!(code, visitor);

    let def_names: Vec<String> = visitor
        .definitions
        .iter()
        .map(|d| d.full_name.clone())
        .collect();

    // All 4 different 'x' variables should exist with different qualified names
    assert!(
        def_names.contains(&"test_module.x".to_owned()),
        "Missing module.x"
    );
    assert!(
        def_names.contains(&"test_module.OuterClass.x".to_owned()),
        "Missing OuterClass.x"
    );
    assert!(
        def_names.contains(&"test_module.OuterClass.outer_method.x".to_owned()),
        "Missing outer_method.x"
    );
    assert!(
        def_names.contains(&"test_module.OuterClass.outer_method.nested_func.x".to_owned()),
        "Missing nested_func.x"
    );

    let x_defs: Vec<_> = def_names
        .iter()
        .filter(|n| n.ends_with(".x") || n == &"test_module.x")
        .collect();
    assert_eq!(
        x_defs.len(),
        4,
        "Expected 4 x definitions at different levels, got: {x_defs:?}"
    );
}
