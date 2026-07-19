//! Tests for raw metrics (LOC, SLOC, comments).

use cytoscnpy::raw_metrics::analyze_raw;

#[test]
fn test_empty_file() {
    let code = "";
    let metrics = analyze_raw(code);
    assert_eq!(metrics.loc, 0);
    assert_eq!(metrics.sloc, 0);
    assert_eq!(metrics.comments, 0);
    assert_eq!(metrics.multi, 0);
    assert_eq!(metrics.blank, 0);
}

#[test]
fn test_only_comments() {
    let code = "# comment 1\n# comment 2";
    let metrics = analyze_raw(code);
    assert_eq!(metrics.loc, 2);
    assert_eq!(metrics.sloc, 0);
    assert_eq!(metrics.comments, 2);
    assert_eq!(metrics.multi, 0);
    assert_eq!(metrics.blank, 0);
}

#[test]
fn test_code_and_comments() {
    let code = "x = 1\n# comment\ny = 2";
    let metrics = analyze_raw(code);
    assert_eq!(metrics.loc, 3);
    assert_eq!(metrics.sloc, 2);
    assert_eq!(metrics.comments, 1);
    assert_eq!(metrics.multi, 0);
    assert_eq!(metrics.blank, 0);
}

#[test]
fn test_docstrings() {
    let code = r#"
def foo():
    """
    This is a docstring.
    """
    pass
"#;
    let metrics = analyze_raw(code);
    // Line 1: blank (if starts with newline) or def foo():
    // Let's count carefully:
    // 1. (blank)
    // 2. def foo():
    // 3.     """
    // 4.     This is a docstring.
    // 5.     """
    // 6.     pass
    // A trailing newline does not create an additional source line.

    // LOC: 6
    // Blank: 1 (line 1)
    // Multi: 3 (lines 3, 4, 5)
    // SLOC: 2 (docstring lines excluded)
    // Comments: 0

    assert_eq!(metrics.loc, 6);
    assert_eq!(metrics.blank, 1);
    assert_eq!(metrics.multi, 3);
    assert_eq!(metrics.sloc, 2);
    assert_eq!(metrics.lloc, 2);
}

#[test]
fn test_mixed_content() {
    let code = r#"
import os

def main():
    # This is a comment
    print("Hello")
    
    """
    Multi-line
    String
    """
    x = 1
"#;
    // 1. (blank)
    // 2. import os
    // 3. (blank)
    // 4. def main():
    // 5.     # This is a comment
    // 6.     print("Hello")
    // 7.     (blank)
    // 8.     """
    // 9.     Multi-line
    // 10.    String
    // 11.    """
    // 12.    x = 1
    // A trailing newline does not create an additional source line.

    // LOC: 12
    // Blank: 3 (1, 3, 7)
    // Comments: 1 (5)
    // Multi: 0 (this is a data string, not a docstring)
    // SLOC: 8 (data-string lines are code)

    let metrics = analyze_raw(code);
    assert_eq!(metrics.loc, 12);
    assert_eq!(metrics.blank, 3);
    assert_eq!(metrics.comments, 1);
    assert_eq!(metrics.multi, 0);
    assert_eq!(metrics.sloc, 8);
    assert_eq!(metrics.lloc, 4);
}

#[test]
fn test_inline_comments_and_trailing_newline() {
    let metrics = analyze_raw("first = 1  # inline\nsecond = 2\n");

    assert_eq!(metrics.loc, 2);
    assert_eq!(metrics.blank, 0);
    assert_eq!(metrics.comments, 1);
    assert_eq!(metrics.single_comments, 0);
    assert_eq!(metrics.sloc, 2);
    assert_eq!(metrics.lloc, 2);
}
