use super::*;

#[test]
fn test_raw_single_line_docstring_as_comment() {
    let code = r#"
def function():
    """ a docstring in a single line counts as a single-line comment """
"#;
    let metrics = analyze_raw(code);

    assert!(metrics.sloc >= 1);
}

#[test]
fn test_raw_concatenated_strings() {
    let code = r#"
def function():
    """ this is not a """ """ docstring because it is concatenated """
"#;
    let metrics = analyze_raw(code);

    // Concatenated strings on same line = code
    assert!(metrics.sloc >= 2);
}

#[test]
fn test_raw_docstring_with_inline_comment() {
    let code = r#"
def function():
    """ this is not a docstring """ # because it also has a comment on the line
"#;
    let metrics = analyze_raw(code);

    assert!(metrics.sloc >= 2);
}

#[test]
fn test_raw_semicolon_in_function() {
    let code = r"
def function():
    pass; pass
";
    let metrics = analyze_raw(code);

    assert!(metrics.sloc >= 2);
}

#[test]
fn test_raw_docstring_with_semicolon() {
    let code = r#"
def function():
    """ doc string """; pass
"#;
    let metrics = analyze_raw(code);

    assert!(metrics.sloc >= 2);
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_raw_trailing_newline() {
    let code = "x = 1\n";
    let metrics = analyze_raw(code);

    assert_eq!(metrics.sloc, 1);
}

#[test]
fn test_raw_multiple_blank_lines() {
    let code = r"
x = 1


y = 2
";
    let metrics = analyze_raw(code);

    assert!(metrics.blank >= 2);
    assert_eq!(metrics.sloc, 2);
}

#[test]
fn test_raw_hash_in_string() {
    // Hash inside string should not be counted as comment
    let code = r#"x = "hello # world""#;
    let metrics = analyze_raw(code);

    assert_eq!(metrics.sloc, 1);
    assert_eq!(metrics.comments, 0, "Hash in string is not a comment");
}

#[test]
fn test_raw_nested_quotes() {
    let code = r#"x = "it's a 'test'""#;
    let metrics = analyze_raw(code);

    assert_eq!(metrics.sloc, 1);
}

#[test]
fn test_raw_raw_string() {
    let code = r#"x = r"\n\t\r""#;
    let metrics = analyze_raw(code);

    assert_eq!(metrics.sloc, 1);
}

#[test]
fn test_raw_fstring() {
    let code = r#"x = f"value is {y}""#;
    let metrics = analyze_raw(code);

    assert_eq!(metrics.sloc, 1);
}

#[test]
fn test_raw_multiline_fstring() {
    let code = r#"
x = f"""
value is {y}
and {z}
"""
"#;
    let metrics = analyze_raw(code);

    assert!(metrics.multi >= 3);
}

#[test]
fn test_raw_class_with_methods() {
    let code = r#"
class MyClass:
    """A simple class."""
    
    def __init__(self, x):
        self.x = x
    
    def method(self):
        """A method."""
        return self.x * 2
"#;
    let metrics = analyze_raw(code);

    assert!(metrics.loc >= 10);
    assert!(metrics.sloc >= 6);
    assert!(metrics.blank >= 2);
}

#[test]
fn test_raw_decorator() {
    let code = r"
@decorator
def function():
    pass
";
    let metrics = analyze_raw(code);

    assert!(metrics.sloc >= 3);
}

#[test]
fn test_raw_multiple_decorators() {
    let code = r"
@decorator1
@decorator2
@decorator3
def function():
    pass
";
    let metrics = analyze_raw(code);

    assert!(metrics.sloc >= 5);
}

#[test]
fn test_raw_lambda() {
    let code = "f = lambda x: x * 2";
    let metrics = analyze_raw(code);

    assert_eq!(metrics.sloc, 1);
}

#[test]
fn test_raw_comprehension() {
    let code = "[x for x in range(10) if x % 2 == 0]";
    let metrics = analyze_raw(code);

    assert_eq!(metrics.sloc, 1);
}

#[test]
fn test_raw_multiline_comprehension() {
    let code = r"
result = [
    x * 2
    for x in range(10)
    if x % 2 == 0
]
";
    let metrics = analyze_raw(code);

    assert!(metrics.sloc >= 5);
}

// =============================================================================
// LINE CONTINUATION TESTS
// =============================================================================

#[test]
fn test_raw_line_continuation_basic() {
    // Backslash line continuation
    let code = r"
x = 1 + \
    2 + \
    3
";
    let metrics = analyze_raw(code);

    // 3 physical lines of code
    assert!(metrics.loc >= 3);
    assert!(metrics.sloc >= 3);
}

#[test]
fn test_raw_line_continuation_with_string() {
    let code = r#"
def foo(n=1):
   """
   Try it with n = 294942: it will take a fairly long time.
   """
   if n <= 1: return 1  # otherwise it will melt the cpu
   string = \
           """
           This is a string not a comment
           """
"#;
    let metrics = analyze_raw(code);

    assert!(metrics.loc >= 10);
    assert!(metrics.sloc >= 5);
}

#[test]
fn test_raw_line_continuation_with_comment() {
    let code = r#"
def foo(n=1):
   """
   Try it with n = 294942: it will take a fairly long time.
   """
   if n <= 1: return 1  # otherwise it will melt the cpu
   string =\
           """
           This is a string not a comment
           """
   test = 0
   # Comment
"#;
    let metrics = analyze_raw(code);

    assert!(metrics.comments >= 1);
    assert!(metrics.sloc >= 6);
}

#[test]
fn test_raw_line_continuation_string_not_doc() {
    let code = r#"
def function():
    " a docstring is a not single-line comment when " \
        # followed by a comment on a another line
"#;
    let metrics = analyze_raw(code);

    assert!(metrics.loc >= 3);
    assert!(metrics.comments >= 1);
}

#[test]
fn test_raw_line_continuation_with_blank() {
    let code = r#"
def function():
    """ docstring continued by blank line is not a single-line comment """ \

    pass
"#;
    let metrics = analyze_raw(code);

    assert!(metrics.blank >= 1);
    assert!(metrics.sloc >= 2);
}
