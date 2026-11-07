//! Call-site classification helpers: pytest fixtures and runtime protocols.

use super::{ast, CytoScnPyVisitor, Expr};
use crate::constants::RUNTIME_PROTOCOL_REF_PREFIX;

impl CytoScnPyVisitor<'_> {
    pub(super) fn is_pytest_usefixtures_call(node: &ast::ExprCall) -> bool {
        match &*node.func {
            Expr::Attribute(attr) if attr.attr.as_str() == "usefixtures" => match &*attr.value {
                Expr::Attribute(inner) => inner.attr.as_str() == "mark",
                Expr::Name(name) => name.id.as_str() == "mark",
                _ => false,
            },
            _ => false,
        }
    }

    pub(super) fn runtime_protocol_name(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Name(name) => Some(name.id.to_string()),
            Expr::Attribute(attr) => {
                if let Expr::Name(base_name) = &*attr.value {
                    Some(format!("{}.{}", base_name.id, attr.attr))
                } else {
                    Some(attr.attr.to_string())
                }
            }
            Expr::Subscript(subscript) => Self::runtime_protocol_name(&subscript.value),
            _ => None,
        }
    }

    pub(super) fn record_runtime_protocol_hints(&mut self, type_expr: &Expr) {
        match type_expr {
            Expr::Tuple(tuple_expr) => {
                for element in &tuple_expr.elts {
                    self.record_runtime_protocol_hints(element);
                }
            }
            _ => {
                if let Some(protocol_name) = Self::runtime_protocol_name(type_expr) {
                    self.add_ref(&format!("{RUNTIME_PROTOCOL_REF_PREFIX}{protocol_name}"));
                }
            }
        }
    }
}
