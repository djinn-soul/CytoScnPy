//! Tests for taint source detection.
//!
//! Checks that various taint sources (input, Flask/Django requests, etc.) are correctly identified.

#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]
use cytoscnpy::taint::sources::check_taint_source;
use cytoscnpy::taint::types::TaintSource;
use cytoscnpy::utils::LineIndex;
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
    let source = "input()";
    let expr = parse_expr(source);
    let line_index = LineIndex::new(source);
    let taint = check_taint_source(&expr, &line_index);
    assert!(taint.is_some());
    assert!(matches!(taint.unwrap().source, TaintSource::Input));
}

#[test]
fn test_flask_request_args() {
    let source = "request.args";
    let expr = parse_expr(source);
    let line_index = LineIndex::new(source);
    let taint = check_taint_source(&expr, &line_index);
    assert!(taint.is_some());
    assert!(matches!(
        taint.unwrap().source,
        TaintSource::FlaskRequest(_)
    ));
}

#[test]
fn test_sys_argv() {
    let source = "sys.argv";
    let expr = parse_expr(source);
    let line_index = LineIndex::new(source);
    let taint = check_taint_source(&expr, &line_index);
    assert!(taint.is_some());
    assert!(matches!(taint.unwrap().source, TaintSource::CommandLine));
}
