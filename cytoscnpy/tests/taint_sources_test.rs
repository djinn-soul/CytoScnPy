//! Tests for taint source detection.
//!
//! Checks that various taint sources (input, Flask/Django requests, etc.) are correctly identified.

#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]
use cytoscnpy::taint::sources::check_taint_source;
use cytoscnpy::taint::types::TaintSource;
use ruff_python_ast::{self as ast, Expr};
use ruff_python_parser::{parse, Mode};

fn parse_expr(source: &str) -> Expr {
    let tree = parse(source, Mode::Expression.into()).unwrap();
    if let ast::Mod::Expression(expr) = tree.into_syntax() {
        *expr.body
    } else {
        panic!("Expected expression")
    }
}

#[test]
fn test_input_source() {
    let expr = parse_expr("input()");
    let taint = check_taint_source(&expr);
    assert!(taint.is_some());
    assert!(matches!(taint.unwrap().source, TaintSource::Input));
}

#[test]
fn test_flask_request_args() {
    let expr = parse_expr("request.args");
    let taint = check_taint_source(&expr);
    assert!(taint.is_some());
    assert!(matches!(
        taint.unwrap().source,
        TaintSource::FlaskRequest(_)
    ));
}

#[test]
fn test_sys_argv() {
    let expr = parse_expr("sys.argv");
    let taint = check_taint_source(&expr);
    assert!(taint.is_some());
    assert!(matches!(taint.unwrap().source, TaintSource::CommandLine));
}
