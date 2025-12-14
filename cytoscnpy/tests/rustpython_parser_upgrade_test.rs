//! Integration tests for ruff_python_parser compatibility.
//!
//! These tests verify that the AST structure and API from `ruff_python_parser`
//! works correctly, particularly around function argument handling.

use ruff_python_ast as ast;
use ruff_python_parser::{parse, Mode};

fn get_first_function_stmts(source: &str) -> ast::StmtFunctionDef {
    let parsed = parse(source, Mode::Module.into()).unwrap();
    let mod_ast = parsed.into_syntax();
    if let ast::Mod::Module(mod_module) = mod_ast {
        if let ast::Stmt::FunctionDef(node) = &mod_module.body[0] {
            return node.clone();
        }
    }
    panic!("Expected Module with FunctionDef");
}

#[test]
fn test_parse_simple_function() {
    let source = "def foo(x): pass";
    let node = get_first_function_stmts(source);
    let params = &node.parameters;

    assert_eq!(params.args.len(), 1);
    assert_eq!(params.posonlyargs.len(), 0);
    assert_eq!(params.kwonlyargs.len(), 0);

    assert_eq!(params.args[0].parameter.name.as_str(), "x");
}

#[test]
fn test_parse_args_defaults() {
    let source = "def foo(x, y=1, z=2): pass";
    let node = get_first_function_stmts(source);
    let params = &node.parameters;

    assert_eq!(params.args.len(), 3);

    assert_eq!(params.args[0].parameter.name.as_str(), "x");
    assert_eq!(params.args[1].parameter.name.as_str(), "y");
    assert_eq!(params.args[2].parameter.name.as_str(), "z");

    assert!(params.args[0].default.is_none());
    assert!(params.args[1].default.is_some());
    assert!(params.args[2].default.is_some());
}

#[test]
fn test_parse_posonly_args() {
    let source = "def foo(x, /, y): pass";
    let node = get_first_function_stmts(source);
    let params = &node.parameters;

    assert_eq!(params.posonlyargs.len(), 1);
    assert_eq!(params.args.len(), 1);

    assert_eq!(params.posonlyargs[0].parameter.name.as_str(), "x");
    assert_eq!(params.args[0].parameter.name.as_str(), "y");
}

#[test]
fn test_parse_kwonly_args() {
    let source = "def foo(x, *, y=3): pass";
    let node = get_first_function_stmts(source);
    let params = &node.parameters;

    assert_eq!(params.args.len(), 1);
    assert_eq!(params.kwonlyargs.len(), 1);

    assert_eq!(params.args[0].parameter.name.as_str(), "x");
    assert_eq!(params.kwonlyargs[0].parameter.name.as_str(), "y");
    assert!(params.kwonlyargs[0].default.is_some());
}

#[test]
fn test_parse_complex_args() {
    let source = "def f(pos1, pos2, /, pos_or_kwd, *, kwd1, kwd2=None): pass";
    let node = get_first_function_stmts(source);
    let params = &node.parameters;

    assert_eq!(params.posonlyargs.len(), 2);
    assert_eq!(params.args.len(), 1);
    assert_eq!(params.kwonlyargs.len(), 2);

    assert_eq!(params.posonlyargs[0].parameter.name.as_str(), "pos1");
    assert_eq!(params.posonlyargs[1].parameter.name.as_str(), "pos2");
    assert_eq!(params.args[0].parameter.name.as_str(), "pos_or_kwd");
    assert_eq!(params.kwonlyargs[0].parameter.name.as_str(), "kwd1");
    assert_eq!(params.kwonlyargs[1].parameter.name.as_str(), "kwd2");

    assert!(params.kwonlyargs[0].default.is_none());
    assert!(params.kwonlyargs[1].default.is_some());
}

#[test]
fn test_parse_varargs_varkw() {
    let source = "def f(*args, **kwargs): pass";
    let node = get_first_function_stmts(source);
    let params = &node.parameters;

    assert!(params.vararg.is_some());
    assert!(params.kwarg.is_some());

    assert_eq!(params.vararg.as_ref().unwrap().name.as_str(), "args");
    assert_eq!(params.kwarg.as_ref().unwrap().name.as_str(), "kwargs");
}

