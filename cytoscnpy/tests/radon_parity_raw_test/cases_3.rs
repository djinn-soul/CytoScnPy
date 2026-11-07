use super::*;

#[test]
fn test_raw_implicit_continuation_parens() {
    // Implicit line continuation with parentheses
    let code = r"
result = (
    1 +
    2 +
    3
)
";
    let metrics = analyze_raw(code);

    assert!(metrics.loc >= 5);
    assert!(metrics.sloc >= 5);
}

#[test]
fn test_raw_implicit_continuation_brackets() {
    // Implicit line continuation with brackets
    let code = r"
my_list = [
    1,
    2,
    3,
]
";
    let metrics = analyze_raw(code);

    assert!(metrics.loc >= 5);
    assert!(metrics.sloc >= 5);
}

#[test]
fn test_raw_implicit_continuation_braces() {
    // Implicit line continuation with braces
    let code = r"
my_dict = {
    'a': 1,
    'b': 2,
}
";
    let metrics = analyze_raw(code);

    assert!(metrics.loc >= 4);
    assert!(metrics.sloc >= 4);
}
