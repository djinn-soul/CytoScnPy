use super::*;

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
fn test_bare_escape_is_not_trusted() {
    let expr = parse_expr("escape(x)");
    if let ast::Expr::Call(call) = expr {
        assert!(!is_sanitizer_call(&call));
    } else {
        panic!("Expected call expression");
    }
}

#[test]
fn test_bare_quote_is_not_trusted() {
    let expr = parse_expr("quote(x)");
    if let ast::Expr::Call(call) = expr {
        assert!(!is_sanitizer_call(&call));
    } else {
        panic!("Expected call expression");
    }
}

#[test]
fn test_qualified_html_escape_is_sanitizer() {
    let expr = parse_expr("html.escape(x)");
    if let ast::Expr::Call(call) = expr {
        assert!(is_sanitizer_call(&call));
    } else {
        panic!("Expected call expression");
    }
}

#[test]
fn test_qualified_shlex_quote_is_sanitizer() {
    let expr = parse_expr("shlex.quote(x)");
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
