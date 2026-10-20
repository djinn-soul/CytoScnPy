//! Decorator-name helpers for pytest fixture and test detection.

use crate::constants::{FIXTURE_DECOR_RE, TEST_DECOR_RE};
use ruff_python_ast::Expr;

pub(super) fn is_fixture_decorator_call(call: &ruff_python_ast::ExprCall) -> bool {
    decorator_name_matches_fixture(&call.func)
}

pub(super) fn decorator_name_matches_fixture(expr: &Expr) -> bool {
    let name = decorator_name(expr);
    !name.is_empty() && FIXTURE_DECOR_RE().is_match(&name)
}

pub(super) fn decorator_name_matches_test(expr: &Expr) -> bool {
    let name = decorator_name(expr);
    !name.is_empty() && TEST_DECOR_RE().is_match(&name)
}

pub(super) fn decorator_name(expr: &Expr) -> String {
    match expr {
        Expr::Name(name) => name.id.to_string(),
        Expr::Attribute(attr) => match &*attr.value {
            Expr::Name(base) => format!("{}.{}", base.id, attr.attr),
            _ => attr.attr.to_string(),
        },
        _ => String::new(),
    }
}

pub(super) fn extract_fixture_alias(call: &ruff_python_ast::ExprCall) -> Option<String> {
    for keyword in &call.arguments.keywords {
        if keyword
            .arg
            .as_ref()
            .map(ruff_python_ast::Identifier::as_str)
            != Some("name")
        {
            continue;
        }
        if let Expr::StringLiteral(s) = &keyword.value {
            let value = s.value.to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}
