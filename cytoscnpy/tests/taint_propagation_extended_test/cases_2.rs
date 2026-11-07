use super::*;

#[test]
fn test_urlquote_is_not_global_sanitizer() {
    let expr = parse_expr("django.utils.http.urlquote(x)");
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
    assert!(names.contains('a'));
    assert!(names.contains('b'));
    assert!(names.contains('c'));
}

#[test]
fn test_get_assigned_name_attribute() {
    let expr = parse_expr("obj.attr");
    let name = get_assigned_name(&expr);
    assert!(name.is_none()); // Attribute access doesn't have a simple name
}
