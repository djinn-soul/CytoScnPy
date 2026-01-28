//! Taint source detection.
//!
//! Identifies where untrusted user input enters the program.

mod attr_checks;
mod call_checks;
mod fastapi;
mod subscript_checks;
mod utils;

use super::types::TaintInfo;
use crate::utils::LineIndex;
use ruff_python_ast::Expr;

use attr_checks::check_attribute_source;
use call_checks::check_call_source;
pub use fastapi::check_fastapi_param;
use subscript_checks::check_subscript_source;

/// Checks if an expression is a taint source and returns the taint info.
pub fn check_taint_source(expr: &Expr, line_index: &LineIndex) -> Option<TaintInfo> {
    match expr {
        // Check for function calls that return tainted data
        Expr::Call(call) => check_call_source(call, line_index),
        // Check for attribute access on request objects
        Expr::Attribute(attr) => check_attribute_source(attr, line_index),
        // Check for subscript on request objects (request.args['key'])
        Expr::Subscript(sub) => check_subscript_source(sub, line_index),
        _ => None,
    }
}
