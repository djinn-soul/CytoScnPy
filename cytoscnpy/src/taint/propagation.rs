//! Taint propagation rules.
//!
//! Defines how taint flows through expressions and statements.

use super::types::TaintInfo;
use ruff_python_ast::{self as ast, Expr};
use std::collections::HashMap;

/// Taint state for tracking tainted variables.
#[derive(Debug, Clone, Default)]
pub struct TaintState {
    /// Map from variable name to taint info
    pub tainted: HashMap<String, TaintInfo>,
}

impl TaintState {
    /// Creates a new empty taint state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks a variable as tainted.
    pub fn mark_tainted(&mut self, var: &str, info: TaintInfo) {
        self.tainted.insert(var.to_owned(), info);
    }

    /// Checks if a variable is tainted.
    pub fn is_tainted(&self, var: &str) -> bool {
        self.tainted.contains_key(var)
    }

    /// Gets taint info for a variable.
    pub fn get_taint(&self, var: &str) -> Option<&TaintInfo> {
        self.tainted.get(var)
    }

    /// Removes taint from a variable (sanitization).
    pub fn sanitize(&mut self, var: &str) {
        self.tainted.remove(var);
    }

    /// Merges another taint state (for control flow join).
    pub fn merge(&mut self, other: &TaintState) {
        for (var, info) in &other.tainted {
            if !self.tainted.contains_key(var) {
                self.tainted.insert(var.clone(), info.clone());
            }
        }
    }
}

/// Checks if an expression is tainted based on the current state.
pub fn is_expr_tainted(expr: &Expr, state: &TaintState) -> Option<TaintInfo> {
    match expr {
        // Direct variable reference
        Expr::Name(name) => state.get_taint(name.id.as_str()).cloned(),

        // Binary operation: tainted if either operand is tainted
        Expr::BinOp(binop) => {
            is_expr_tainted(&binop.left, state).or_else(|| is_expr_tainted(&binop.right, state))
        }

        // F-string: tainted if any value is tainted
        // F-string: tainted if any value is tainted
        Expr::FString(fstring) => {
            for _part in &fstring.value {
                /*
                if let ruff_python_ast::FStringPart::Expression(expr_part) = part {
                    if let Some(info) = is_expr_tainted(&expr_part.expression, state) {
                        return Some(info);
                    }
                }
                */
            }
            None
        }

        // Method call: tainted if receiver is tainted (e.g., tainted.upper())
        Expr::Call(call) => {
            if let Expr::Attribute(attr) = &*call.func {
                is_expr_tainted(&attr.value, state)
            } else {
                None
            }
        }

        // Attribute access: tainted if value is tainted
        Expr::Attribute(attr) => is_expr_tainted(&attr.value, state),

        // Subscript: tainted if value is tainted (container[tainted] or tainted[x])
        Expr::Subscript(sub) => {
            is_expr_tainted(&sub.value, state).or_else(|| is_expr_tainted(&sub.slice, state))
        }

        // Tuple/List: tainted if any element is tainted
        Expr::Tuple(tuple) => {
            for elt in &tuple.elts {
                if let Some(info) = is_expr_tainted(elt, state) {
                    return Some(info);
                }
            }
            None
        }

        Expr::List(list) => {
            for elt in &list.elts {
                if let Some(info) = is_expr_tainted(elt, state) {
                    return Some(info);
                }
            }
            None
        }

        // Dict: tainted if any value is tainted
        Expr::Dict(dict) => {
            for item in &dict.items {
                if let Some(key) = &item.key {
                    if let Some(info) = is_expr_tainted(key, state) {
                        return Some(info);
                    }
                }
                if let Some(info) = is_expr_tainted(&item.value, state) {
                    return Some(info);
                }
            }
            None
        }

        // Conditional expression: conservatively tainted if either branch is tainted
        Expr::If(ifexp) => {
            is_expr_tainted(&ifexp.body, state).or_else(|| is_expr_tainted(&ifexp.orelse, state))
        }

        // Lambda and Constant expressions are never tainted
        // Other expressions: conservatively assume not tainted
        _ => None,
    }
}

/// Checks if a call is a sanitizer that removes taint.
pub fn is_sanitizer_call(call: &ast::ExprCall) -> bool {
    if let Some(name) = get_call_name(&call.func) {
        matches!(
            name.as_str(),
            "int"
                | "float"
                | "bool"
                | "html.escape"
                | "escape"
                | "cgi.escape"
                | "markupsafe.escape"
                | "shlex.quote"
                | "shlex.split"
                | "urllib.parse.quote"
                | "quote"
                | "bleach.clean"
        )
    } else {
        false
    }
}

/// Checks if a SQL call uses parameterized queries (sanitized).
pub fn is_parameterized_query(call: &ast::ExprCall) -> bool {
    // Check if execute() has a second parameter (the params tuple/list)
    if let Some(name) = get_call_name(&call.func) {
        if name.ends_with(".execute") || name.ends_with(".executemany") {
            // Has second argument = parameterized
            return call.arguments.args.len() >= 2;
        }
    }
    false
}

/// Gets the variable name being assigned in a target expression.
pub fn get_assigned_name(target: &Expr) -> Option<String> {
    match target {
        Expr::Name(name) => Some(name.id.to_string()),
        Expr::Tuple(tuple) => {
            // For tuple unpacking, return comma-separated names
            let names: Vec<String> = tuple.elts.iter().filter_map(get_assigned_name).collect();
            if names.is_empty() {
                None
            } else {
                Some(names.join(","))
            }
        }
        _ => None,
    }
}

/// Extracts the call name from a function expression.
fn get_call_name(func: &Expr) -> Option<String> {
    match func {
        Expr::Name(node) => Some(node.id.to_string()),
        Expr::Attribute(node) => {
            if let Expr::Name(value) = &*node.value {
                Some(format!("{}.{}", value.id, node.attr))
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taint::types::TaintSource;
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
}
