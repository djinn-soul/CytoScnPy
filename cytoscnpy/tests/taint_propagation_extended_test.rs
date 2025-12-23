//! Extended tests for `taint/propagation.rs` - Taint propagation rules.
#![allow(clippy::unwrap_used)]

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

// ============================================================================
// TaintState Tests
// ============================================================================

#[test]
fn test_taint_state_new() {
    let state = TaintState::new();
    assert!(state.tainted.is_empty());
}

#[test]
fn test_taint_state_default() {
    let state = TaintState::default();
    assert!(state.tainted.is_empty());
}

#[test]
fn test_taint_state_mark_tainted() {
    let mut state = TaintState::new();
    let info = TaintInfo::new(TaintSource::Input, 1);
    state.mark_tainted("x", info);
    assert!(state.is_tainted("x"));
    assert!(!state.is_tainted("y"));
}

#[test]
fn test_taint_state_get_taint() {
    let mut state = TaintState::new();
    let info = TaintInfo::new(TaintSource::Input, 1);
    state.mark_tainted("x", info);

    let taint = state.get_taint("x");
    assert!(taint.is_some());
    assert_eq!(taint.unwrap().source, TaintSource::Input);

    let no_taint = state.get_taint("y");
    assert!(no_taint.is_none());
}

#[test]
fn test_taint_state_sanitize() {
    let mut state = TaintState::new();
    let info = TaintInfo::new(TaintSource::Input, 1);
    state.mark_tainted("x", info);
    assert!(state.is_tainted("x"));

    state.sanitize("x");
    assert!(!state.is_tainted("x"));
}

#[test]
fn test_taint_state_merge() {
    let mut state1 = TaintState::new();
    let mut state2 = TaintState::new();

    state1.mark_tainted("x", TaintInfo::new(TaintSource::Input, 1));
    state2.mark_tainted("y", TaintInfo::new(TaintSource::CommandLine, 2));
    state2.mark_tainted("x", TaintInfo::new(TaintSource::CommandLine, 3)); // Duplicate

    state1.merge(&state2);

    assert!(state1.is_tainted("x"));
    assert!(state1.is_tainted("y"));
    // Original x value should be preserved (not overwritten)
    assert_eq!(state1.get_taint("x").unwrap().source, TaintSource::Input);
}

// ============================================================================
// is_expr_tainted Tests
// ============================================================================

#[test]
fn test_is_expr_tainted_name() {
    let mut state = TaintState::new();
    state.mark_tainted("x", TaintInfo::new(TaintSource::Input, 1));

    let expr = parse_expr("x");
    let result = is_expr_tainted(&expr, &state);
    assert!(result.is_some());
}

#[test]
fn test_is_expr_tainted_name_not_tainted() {
    let state = TaintState::new();

    let expr = parse_expr("x");
    let result = is_expr_tainted(&expr, &state);
    assert!(result.is_none());
}

#[test]
fn test_is_expr_tainted_binop_left() {
    let mut state = TaintState::new();
    state.mark_tainted("x", TaintInfo::new(TaintSource::Input, 1));

    let expr = parse_expr("x + y");
    let result = is_expr_tainted(&expr, &state);
    assert!(result.is_some());
}

#[test]
fn test_is_expr_tainted_binop_right() {
    let mut state = TaintState::new();
    state.mark_tainted("y", TaintInfo::new(TaintSource::Input, 1));

    let expr = parse_expr("x + y");
    let result = is_expr_tainted(&expr, &state);
    assert!(result.is_some());
}

#[test]
fn test_is_expr_tainted_binop_clean() {
    let state = TaintState::new();

    let expr = parse_expr("x + y");
    let result = is_expr_tainted(&expr, &state);
    assert!(result.is_none());
}

#[test]
fn test_is_expr_tainted_attribute() {
    let mut state = TaintState::new();
    state.mark_tainted("obj", TaintInfo::new(TaintSource::Input, 1));

    let expr = parse_expr("obj.attr");
    let result = is_expr_tainted(&expr, &state);
    assert!(result.is_some());
}

#[test]
fn test_is_expr_tainted_subscript_value() {
    let mut state = TaintState::new();
    state.mark_tainted("arr", TaintInfo::new(TaintSource::Input, 1));

    let expr = parse_expr("arr[0]");
    let result = is_expr_tainted(&expr, &state);
    assert!(result.is_some());
}

#[test]
fn test_is_expr_tainted_subscript_slice() {
    let mut state = TaintState::new();
    state.mark_tainted("idx", TaintInfo::new(TaintSource::Input, 1));

    let expr = parse_expr("arr[idx]");
    let result = is_expr_tainted(&expr, &state);
    assert!(result.is_some());
}

#[test]
fn test_is_expr_tainted_tuple() {
    let mut state = TaintState::new();
    state.mark_tainted("x", TaintInfo::new(TaintSource::Input, 1));

    let expr = parse_expr("(a, x, b)");
    let result = is_expr_tainted(&expr, &state);
    assert!(result.is_some());
}

#[test]
fn test_is_expr_tainted_list() {
    let mut state = TaintState::new();
    state.mark_tainted("x", TaintInfo::new(TaintSource::Input, 1));

    let expr = parse_expr("[a, b, x]");
    let result = is_expr_tainted(&expr, &state);
    assert!(result.is_some());
}

#[test]
fn test_is_expr_tainted_dict() {
    let mut state = TaintState::new();
    state.mark_tainted("v", TaintInfo::new(TaintSource::Input, 1));

    let expr = parse_expr("{'key': v}");
    let result = is_expr_tainted(&expr, &state);
    assert!(result.is_some());
}

#[test]
fn test_is_expr_tainted_conditional() {
    let mut state = TaintState::new();
    state.mark_tainted("x", TaintInfo::new(TaintSource::Input, 1));

    let expr = parse_expr("x if cond else y");
    let result = is_expr_tainted(&expr, &state);
    assert!(result.is_some());
}

#[test]
fn test_is_expr_tainted_constant() {
    let state = TaintState::new();

    let expr = parse_expr("42");
    let result = is_expr_tainted(&expr, &state);
    assert!(result.is_none());
}

#[test]
fn test_is_expr_tainted_string_literal() {
    let state = TaintState::new();

    let expr = parse_expr("'hello'");
    let result = is_expr_tainted(&expr, &state);
    assert!(result.is_none());
}

// ============================================================================
// is_sanitizer_call Tests
// ============================================================================

#[test]
fn test_is_sanitizer_int() {
    let expr = parse_expr("int(x)");
    if let ast::Expr::Call(call) = expr {
        assert!(is_sanitizer_call(&call));
    } else {
        panic!("Expected call expression");
    }
}

#[test]
fn test_is_sanitizer_float() {
    let expr = parse_expr("float(x)");
    if let ast::Expr::Call(call) = expr {
        assert!(is_sanitizer_call(&call));
    } else {
        panic!("Expected call expression");
    }
}

#[test]
fn test_is_sanitizer_bool() {
    let expr = parse_expr("bool(x)");
    if let ast::Expr::Call(call) = expr {
        assert!(is_sanitizer_call(&call));
    } else {
        panic!("Expected call expression");
    }
}

#[test]
fn test_is_sanitizer_escape() {
    let expr = parse_expr("escape(x)");
    if let ast::Expr::Call(call) = expr {
        assert!(is_sanitizer_call(&call));
    } else {
        panic!("Expected call expression");
    }
}

#[test]
fn test_is_sanitizer_quote() {
    let expr = parse_expr("quote(x)");
    if let ast::Expr::Call(call) = expr {
        assert!(is_sanitizer_call(&call));
    } else {
        panic!("Expected call expression");
    }
}

#[test]
fn test_is_not_sanitizer() {
    let expr = parse_expr("dangerous(x)");
    if let ast::Expr::Call(call) = expr {
        assert!(!is_sanitizer_call(&call));
    } else {
        panic!("Expected call expression");
    }
}

// ============================================================================
// is_parameterized_query Tests
// ============================================================================

#[test]
fn test_parameterized_query_with_params() {
    let expr = parse_expr("cursor.execute(query, params)");
    if let ast::Expr::Call(call) = expr {
        assert!(is_parameterized_query(&call));
    } else {
        panic!("Expected call expression");
    }
}

#[test]
fn test_parameterized_query_without_params() {
    let expr = parse_expr("cursor.execute(query)");
    if let ast::Expr::Call(call) = expr {
        assert!(!is_parameterized_query(&call));
    } else {
        panic!("Expected call expression");
    }
}

#[test]
fn test_non_execute_call() {
    let expr = parse_expr("cursor.fetch()");
    if let ast::Expr::Call(call) = expr {
        assert!(!is_parameterized_query(&call));
    } else {
        panic!("Expected call expression");
    }
}

// ============================================================================
// get_assigned_name Tests
// ============================================================================

#[test]
fn test_get_assigned_name_simple() {
    let expr = parse_expr("x");
    let name = get_assigned_name(&expr);
    assert_eq!(name, Some("x".to_owned()));
}

#[test]
fn test_get_assigned_name_tuple() {
    let expr = parse_expr("(a, b, c)");
    let name = get_assigned_name(&expr);
    assert!(name.is_some());
    let names = name.unwrap();
    assert!(names.contains("a"));
    assert!(names.contains("b"));
    assert!(names.contains("c"));
}

#[test]
fn test_get_assigned_name_attribute() {
    let expr = parse_expr("obj.attr");
    let name = get_assigned_name(&expr);
    assert!(name.is_none()); // Attribute access doesn't have a simple name
}
