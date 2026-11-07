//! Extended tests for `taint/sinks.rs` - Dangerous sink detection.
#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]

use cytoscnpy::taint::sinks::{check_sink, SINK_PATTERNS};
use cytoscnpy::taint::types::{Severity, VulnType};
use ruff_python_ast as ast;
use ruff_python_parser::parse_expression;

/// Parse a call expression for testing.
fn parse_call(source: &str) -> ast::ExprCall {
    let expr = parse_expression(source).unwrap().into_expr();
    if let ast::Expr::Call(call) = expr {
        call
    } else {
        panic!("Expected call expression, got: {source}");
    }
}

#[path = "taint_sinks_extended_test/cases_1.rs"]
mod cases_1;
