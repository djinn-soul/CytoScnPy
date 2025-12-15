//! Tests for taint propagation logic.
//!
//! Verifies that taint is correctly propogated through variable assignments, binary operations, etc.

#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]
use cytoscnpy::taint::propagation::{is_expr_tainted, TaintState};
use cytoscnpy::taint::types::{TaintInfo, TaintSource};
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
fn test_tainted_variable_propagation() {
    let mut state = TaintState::new();
    state.mark_tainted("x", TaintInfo::new(TaintSource::Input, 1));

    let expr = parse_expr("x");
    assert!(is_expr_tainted(&expr, &state).is_some());
}

#[test]
fn test_binop_propagation() {
    let mut state = TaintState::new();
    state.mark_tainted("x", TaintInfo::new(TaintSource::Input, 1));

    let expr = parse_expr("x + 'suffix'");
    assert!(is_expr_tainted(&expr, &state).is_some());
}

#[test]
fn test_clean_variable() {
    let state = TaintState::new();
    let expr = parse_expr("clean_var");
    assert!(is_expr_tainted(&expr, &state).is_none());
}
