//! Extended tests for `taint/propagation.rs` - Taint propagation rules.
#![allow(clippy::unwrap_used, clippy::panic)]

use cytoscnpy::taint::propagation::{
    get_assigned_name, is_expr_tainted, is_parameterized_query, is_sanitizer_call, TaintState,
};
use cytoscnpy::taint::types::{TaintInfo, TaintSource};

use ruff_python_ast as ast;
use ruff_python_parser::parse_expression;

/// Parse an expression for testing.
fn parse_expr(source: &str) -> ast::Expr {
    parse_expression(source).unwrap().into_expr()
}

#[path = "taint_propagation_extended_test/cases_1.rs"]
mod cases_1;
#[path = "taint_propagation_extended_test/cases_2.rs"]
mod cases_2;
