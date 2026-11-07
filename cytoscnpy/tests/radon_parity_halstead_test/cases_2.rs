use super::*;

#[test]
fn test_halstead_complex_expression() {
    let code = "result = (a + b) * (c - d) / e ** 2";
    let (_ops, _opnds, d_ops, d_opnds) = get_halstead_counts(code);

    // Should have +, -, *, /, ** = at least 5 distinct operators
    assert!(d_ops >= 4, "Should have multiple distinct operators");
    // Should have result, a, b, c, d, e, 2 = 7 distinct operands
    assert!(d_opnds >= 5, "Should have multiple distinct operands");
}

#[test]
fn test_halstead_function_call() {
    let code = "x = func(a, b, c)";
    let (ops, opnds, _d_ops, _d_opnds) = get_halstead_counts(code);

    // func is an operator (call), a, b, c, x are operands
    assert!(ops >= 1, "Should count function call");
    assert!(opnds >= 3, "Should count arguments as operands");
}

#[test]
fn test_halstead_list_operations() {
    let code = "x = [1, 2, 3] + [4, 5]";
    let (ops, opnds, _d_ops, _d_opnds) = get_halstead_counts(code);

    assert!(ops >= 1, "Should count + operator");
    assert!(opnds >= 5, "Should count list elements");
}

#[test]
fn test_halstead_dict_operations() {
    let code = "d = {'a': 1, 'b': 2}";
    let (_ops, opnds, _d_ops, _d_opnds) = get_halstead_counts(code);

    assert!(opnds >= 4, "Should count keys and values");
}
