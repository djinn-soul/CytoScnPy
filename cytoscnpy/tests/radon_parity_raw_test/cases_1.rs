use super::*;

// =============================================================================
// LOGICAL LINES (LLOC) TESTS
// =============================================================================

#[test]
fn test_raw_empty() {
    let code = "";
    let metrics = analyze_raw(code);

    assert_eq!(metrics.loc, 0);
    assert_eq!(metrics.sloc, 0);
    assert_eq!(metrics.comments, 0);
    assert_eq!(metrics.multi, 0);
    assert_eq!(metrics.blank, 0);
}

#[test]
fn test_raw_only_comment() {
    let code = "# most useless comment";
    let metrics = analyze_raw(code);

    assert_eq!(metrics.sloc, 0, "Comment line should not count as SLOC");
    assert_eq!(metrics.comments, 1);
}

#[test]
fn test_raw_simple_expression() {
    let code = "a * b + c";
    let metrics = analyze_raw(code);

    assert_eq!(metrics.sloc, 1);
}

#[test]
fn test_raw_if_statement() {
    let code = "if a:";
    let metrics = analyze_raw(code);

    assert_eq!(metrics.sloc, 1);
}

#[test]
fn test_raw_if_with_comment() {
    let code = "if a:  # just a comment";
    let metrics = analyze_raw(code);

    assert_eq!(
        metrics.sloc, 1,
        "Code line with inline comment is still SLOC"
    );
}

#[test]
fn test_raw_if_pass_single_line() {
    let code = "if a: pass";
    let metrics = analyze_raw(code);

    // This is 1 line but 2 logical statements
    assert_eq!(metrics.loc, 1);
    assert_eq!(metrics.sloc, 1);
}

#[test]
fn test_raw_expression_with_comment() {
    let code = "42 # a comment";
    let metrics = analyze_raw(code);

    assert_eq!(metrics.sloc, 1, "Line with code and comment is SLOC");
}

#[test]
fn test_raw_multiline_string() {
    let code = r#"
"""
multiple
"""
"#;
    let metrics = analyze_raw(code);

    assert!(metrics.multi >= 3, "Should count multi-line string lines");
}

#[test]
fn test_raw_just_comment() {
    let code = "# just a comment";
    let metrics = analyze_raw(code);

    assert_eq!(metrics.sloc, 0);
    assert_eq!(metrics.comments, 1);
}

#[test]
fn test_raw_semicolon_statements() {
    let code = "a = 2; b = 43";
    let metrics = analyze_raw(code);

    // Single line, but could be 2 logical lines
    assert_eq!(metrics.loc, 1);
    assert_eq!(metrics.sloc, 1);
}

#[test]
fn test_raw_multiple_semicolon_statements() {
    let code = "a = 2; b = 43; c = 42; d = 3; print(a)";
    let metrics = analyze_raw(code);

    assert_eq!(metrics.loc, 1);
    assert_eq!(metrics.sloc, 1);
}

// =============================================================================
// ANALYZE CASES - Full module analysis
// =============================================================================

#[test]
fn test_raw_docstring_only() {
    let code = r#"
"""
doc?
"""
"#;
    let metrics = analyze_raw(code);

    // LOC: 4 (including blank lines)
    // Multi: 3 (the docstring lines)
    assert!(metrics.loc >= 3);
    assert!(metrics.multi >= 3);
}

#[test]
fn test_raw_mixed_content() {
    let code = r#"
# just a comment
if a and b:
    print('woah')
else:
    # you'll never get here
    print('ven')
"#;
    let metrics = analyze_raw(code);

    // Should have code, comments, and no multi-line strings
    assert!(metrics.sloc >= 4);
    assert!(metrics.comments >= 2);
    assert_eq!(metrics.multi, 0);
}

#[test]
fn test_raw_all_comments() {
    let code = r"
#
#
#
";
    let metrics = analyze_raw(code);

    assert_eq!(metrics.comments, 3);
    assert_eq!(metrics.sloc, 0);
}

#[test]
fn test_raw_blanks_in_control() {
    let code = r"
if a:
    print


else:
    print
";
    let metrics = analyze_raw(code);

    assert!(metrics.blank >= 2);
    assert!(metrics.sloc >= 4);
}

#[test]
fn test_raw_function_with_docstring() {
    let code = r#"
def f(n):
    """here"""
    return n * f(n - 1)
"#;
    let metrics = analyze_raw(code);

    // Single-line docstring might be counted differently
    assert!(metrics.sloc >= 2);
}

#[test]
fn test_raw_complex_function() {
    let code = r#"
def hip(a, k):
    if k == 1: return a
    # getting high...
    return a ** hip(a, k - 1)

def fib(n):
    """Compute the n-th Fibonacci number.

    Try it with n = 294942: it will take a fairly long time.
    """
    if n <= 1: return 1  # otherwise it will melt the cpu
    return fib(n - 2) + fib(n - 1)
"#;
    let metrics = analyze_raw(code);

    // Complex file with functions, comments, docstrings
    assert!(metrics.loc >= 12);
    assert!(metrics.sloc >= 6);
    assert!(metrics.comments >= 1);
    assert!(metrics.multi >= 3);
}

#[test]
fn test_raw_function_with_default_param() {
    let code = r#"
def foo(n=1):
   """
   Try it with n = 294942: it will take a fairly long time.
   """
   if n <= 1: return 1  # otherwise it will melt the cpu
"#;
    let metrics = analyze_raw(code);

    assert!(metrics.multi >= 3, "Should count docstring lines");
}

#[test]
fn test_raw_multiline_string_not_doc() {
    let code = r#"
def foo(n=1):
   """
   Try it with n = 294942: it will take a fairly long time.
   """
   if n <= 1: return 1  # otherwise it will melt the cpu
   string = """This is a string not a comment"""
"#;
    let metrics = analyze_raw(code);

    // The inline string should not add to multi
    assert!(metrics.sloc >= 3);
}

#[test]
fn test_raw_multiline_string_variable() {
    let code = r#"
def foo(n=1):
   """
   Try it with n = 294942: it will take a fairly long time.
   """
   if n <= 1: return 1  # otherwise it will melt the cpu
   string = """
            This is a string not a comment
            """
"#;
    let metrics = analyze_raw(code);

    // Multi-line string assigned to variable should count
    assert!(metrics.multi >= 3);
}

#[test]
fn test_raw_multiline_function_signature() {
    let code = r#"
def function(
    args
):
    """This is a multi-line docstring
    for the function
    """
    pass
"#;
    let metrics = analyze_raw(code);

    assert!(metrics.loc >= 7);
    assert!(metrics.sloc >= 3);
}

#[test]
fn test_raw_inline_multiline_string() {
    let code = r#"
def function():
    multiline_with_equals_in_it = """ """
    pass
"#;
    let metrics = analyze_raw(code);

    // Single-line triple-quoted string
    assert!(metrics.sloc >= 3);
}
