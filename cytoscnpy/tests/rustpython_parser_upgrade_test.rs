//! Integration tests for rustpython-parser upgrade compatibility.
//!
//! These tests verify that the AST structure and API from `rustpython-parser` 0.4.x
//! works correctly, particularly around function argument handling.

use rustpython_ast as ast;
use rustpython_parser::{parse, Mode};

fn get_first_function_stmts(source: &str) -> ast::StmtFunctionDef {
    let mod_ast = parse(source, Mode::Module, "<test>").unwrap();
    // 0.3.0: Mod::Module(ModModule) -> mod_module.body
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
    let args = &node.args;

    assert_eq!(args.args.len(), 1);
    assert_eq!(args.posonlyargs.len(), 0);
    assert_eq!(args.kwonlyargs.len(), 0);

    assert_eq!(args.args[0].def.arg.as_str(), "x");
}

#[test]
fn test_parse_args_defaults() {
    let source = "def foo(x, y=1, z=2): pass";
    let node = get_first_function_stmts(source);
    let args = &node.args;

    assert_eq!(args.args.len(), 3);

    assert_eq!(args.args[0].def.arg.as_str(), "x");
    assert_eq!(args.args[1].def.arg.as_str(), "y");
    assert_eq!(args.args[2].def.arg.as_str(), "z");

    assert!(args.args[0].default.is_none());
    assert!(args.args[1].default.is_some());
    assert!(args.args[2].default.is_some());
}

#[test]
fn test_parse_posonly_args() {
    let source = "def foo(x, /, y): pass";
    let node = get_first_function_stmts(source);
    let args = &node.args;

    assert_eq!(args.posonlyargs.len(), 1);
    assert_eq!(args.args.len(), 1);

    assert_eq!(args.posonlyargs[0].def.arg.as_str(), "x");
    assert_eq!(args.args[0].def.arg.as_str(), "y");
}

#[test]
fn test_parse_kwonly_args() {
    let source = "def foo(x, *, y=3): pass";
    let node = get_first_function_stmts(source);
    let args = &node.args;

    assert_eq!(args.args.len(), 1);
    assert_eq!(args.kwonlyargs.len(), 1);

    assert_eq!(args.args[0].def.arg.as_str(), "x");
    assert_eq!(args.kwonlyargs[0].def.arg.as_str(), "y");
    assert!(args.kwonlyargs[0].default.is_some());
}

#[test]
fn test_parse_complex_args() {
    let source = "def f(pos1, pos2, /, pos_or_kwd, *, kwd1, kwd2=None): pass";
    let node = get_first_function_stmts(source);
    let args = &node.args;

    assert_eq!(args.posonlyargs.len(), 2);
    assert_eq!(args.args.len(), 1);
    assert_eq!(args.kwonlyargs.len(), 2);

    assert_eq!(args.posonlyargs[0].def.arg.as_str(), "pos1");
    assert_eq!(args.posonlyargs[1].def.arg.as_str(), "pos2");
    assert_eq!(args.args[0].def.arg.as_str(), "pos_or_kwd");
    assert_eq!(args.kwonlyargs[0].def.arg.as_str(), "kwd1");
    assert_eq!(args.kwonlyargs[1].def.arg.as_str(), "kwd2");

    assert!(args.kwonlyargs[0].default.is_none());
    assert!(args.kwonlyargs[1].default.is_some());
}

#[test]
fn test_parse_varargs_varkw() {
    let source = "def f(*args, **kwargs): pass";
    let node = get_first_function_stmts(source);
    let args = &node.args;

    assert!(args.vararg.is_some());
    assert!(args.kwarg.is_some());

    assert_eq!(args.vararg.as_ref().unwrap().arg.as_str(), "args");
    assert_eq!(args.kwarg.as_ref().unwrap().arg.as_str(), "kwargs");
}
