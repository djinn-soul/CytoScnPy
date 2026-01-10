//! Utility functions for taint source detection.

use ruff_python_ast::Expr;

/// Extracts the call name from a function expression.
pub(crate) fn get_call_name(func: &Expr) -> Option<String> {
    match func {
        Expr::Name(node) => Some(node.id.to_string()),
        Expr::Attribute(node) => {
            if let Expr::Name(value) = &*node.value {
                Some(format!("{}.{}", value.id, node.attr))
            } else if let Expr::Attribute(inner) = &*node.value {
                // Handle chained attributes like request.args.get
                if let Expr::Name(name) = &*inner.value {
                    Some(format!("{}.{}.{}", name.id, inner.attr, node.attr))
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}
