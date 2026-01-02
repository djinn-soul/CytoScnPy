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
    // 7. (blank)

    // LOC: 7
    // Blank: 2 (lines 1 and 7)
    // Multi: 3 (lines 3, 4, 5)
    // SLOC: 2 (lines 2 and 6)
    // Comments: 0

    assert_eq!(metrics.loc, 7);
    assert_eq!(metrics.blank, 2);
    assert_eq!(metrics.multi, 3);
    assert_eq!(metrics.sloc, 2);
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
    // 13. (blank)

    // LOC: 13
    // Blank: 4 (1, 3, 7, 13)
    // Comments: 1 (5)
    // Multi: 4 (8, 9, 10, 11)
    // SLOC: 4 (2, 4, 6, 12)

    let metrics = analyze_raw(code);
    assert_eq!(metrics.loc, 13);
    assert_eq!(metrics.blank, 4);
    assert_eq!(metrics.comments, 1);
    assert_eq!(metrics.multi, 4);
    assert_eq!(metrics.sloc, 4);
}
