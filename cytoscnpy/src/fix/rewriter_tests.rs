use super::*;
use crate::fix::EditBuilder;

#[test]
fn verified_rewrite_returns_syntax_errors() {
    let mut rewriter = ByteRangeRewriter::new("x=1\n");
    rewriter.add_edit(Edit::new(0, 3, "if"));
    let error = rewriter.apply_verified().unwrap_err();
    assert!(matches!(error, RewriteError::InvalidPython { .. }));
    assert!(error.to_string().contains("Invalid Python after rewrite"));
    assert!(error.to_string().contains("Expected"));
}

#[test]
fn verified_rewrite_accepts_valid_python() {
    let mut rewriter = ByteRangeRewriter::new("x=1\n");
    rewriter.add_edit(Edit::new(2, 3, "2"));
    assert_eq!(rewriter.apply_verified().unwrap(), "x=2\n");
    assert!(ByteRangeRewriter::new("if\n").apply_verified().is_err());
}

#[test]
fn invalid_ranges_return_errors_without_panicking() {
    for (source, start, end) in [("abc", 2, 1), ("é", 0, 1), ("é", 1, 2), ("abc", 8, 8)] {
        let mut rewriter = ByteRangeRewriter::new(source);
        rewriter.add_edit(Edit::delete(start, end));
        assert!(rewriter.validate().is_err());
        assert!(rewriter.apply().is_err());
    }
}

#[test]
fn unicode_edits_and_insertion_at_end_are_valid() {
    let mut rewriter = ByteRangeRewriter::new("é🙂");
    rewriter.add_edit(Edit::new(0, 2, "ê"));
    rewriter.add_edit(Edit::insert(6, "!"));
    assert_eq!(rewriter.apply().unwrap(), "ê🙂!");
}

#[test]
fn test_simple_replacement() {
    let source = "hello world";
    let mut rewriter = ByteRangeRewriter::new(source);
    rewriter.add_edit(Edit::new(0, 5, "hi"));

    let result = rewriter.apply().expect("should apply");
    assert_eq!(result, "hi world");
}

#[test]
fn test_multiple_non_overlapping_edits() {
    let source = "aaa bbb ccc";
    let mut rewriter = ByteRangeRewriter::new(source);
    rewriter.add_edit(Edit::new(0, 3, "AAA"));
    rewriter.add_edit(Edit::new(8, 11, "CCC"));

    let result = rewriter.apply().expect("should apply");
    assert_eq!(result, "AAA bbb CCC");
}

#[test]
fn test_overlapping_edits_error() {
    let source = "hello world";
    let mut rewriter = ByteRangeRewriter::new(source);
    rewriter.add_edit(Edit::new(0, 8, "hi"));
    rewriter.add_edit(Edit::new(5, 10, "there"));

    let result = rewriter.apply();
    assert!(matches!(result, Err(RewriteError::OverlappingEdits { .. })));
}

#[test]
fn test_out_of_bounds_error() {
    let source = "short";
    let mut rewriter = ByteRangeRewriter::new(source);
    rewriter.add_edit(Edit::new(0, 100, "long"));

    let result = rewriter.apply();
    assert!(matches!(result, Err(RewriteError::OutOfBounds { .. })));
}

#[test]
fn test_deletion() {
    let source = "hello world";
    let mut rewriter = ByteRangeRewriter::new(source);
    rewriter.add_edit(Edit::delete(5, 11));

    let result = rewriter.apply().expect("should apply");
    assert_eq!(result, "hello");
}

#[test]
fn test_insertion() {
    let source = "hello world";
    let mut rewriter = ByteRangeRewriter::new(source);
    rewriter.add_edit(Edit::insert(5, " beautiful"));

    let result = rewriter.apply().expect("should apply");
    assert_eq!(result, "hello beautiful world");
}

#[test]
fn test_edit_builder() {
    let edits = EditBuilder::new()
        .replace(0, 5, "hi")
        .delete(6, 12)
        .insert(6, "there")
        .build();

    assert_eq!(edits.len(), 3);
}

#[test]
fn test_python_function_deletion() {
    let source = r#"def used_func():
    return "used"

def unused_func():
    return "unused"

def another_used():
    pass
"#;
    // Delete lines 4-5 (unused_func)
    let mut rewriter = ByteRangeRewriter::new(source);
    // Calculate byte range for unused_func (starts at byte 35, ends at byte 72)
    let start = source
        .find("def unused_func")
        .expect("Should find unused_func");
    let end = source
        .find("def another_used")
        .expect("Should find another_used");
    rewriter.add_edit(Edit::delete(start, end));

    let result = rewriter.apply().expect("should apply");
    assert!(result.contains("def used_func"));
    assert!(!result.contains("def unused_func"));
    assert!(result.contains("def another_used"));
}

#[test]
fn test_python_import_deletion() {
    let source = r"import os
import sys
from typing import List

def main():
    print(sys.version)
";
    // Delete "import os\n" (bytes 0-10)
    let mut rewriter = ByteRangeRewriter::new(source);
    let end = source.find("import sys").expect("Should find import sys");
    rewriter.add_edit(Edit::delete(0, end));

    let result = rewriter.apply().expect("should apply");
    assert!(!result.contains("import os"));
    assert!(result.contains("import sys"));
}

#[test]
fn test_multiple_deletions_same_file() {
    let source = r"import os
import sys
import json

def func_a():
    pass

def func_b():
    pass

def func_c():
    pass
";
    // Delete import os and func_b
    let mut rewriter = ByteRangeRewriter::new(source);

    // Delete import os line
    let os_end = source.find("import sys").expect("Should find import sys");
    rewriter.add_edit(Edit::delete(0, os_end));

    // Delete func_b
    let func_b_start = source.find("def func_b").expect("Should find func_b");
    let func_b_end = source.find("def func_c").expect("Should find func_c");
    rewriter.add_edit(Edit::delete(func_b_start, func_b_end));

    let result = rewriter.apply().expect("should apply");
    assert!(!result.contains("import os"));
    assert!(result.contains("import sys"));
    assert!(result.contains("def func_a"));
    assert!(!result.contains("def func_b"));
    assert!(result.contains("def func_c"));
}

#[test]
fn test_preserves_formatting() {
    let source = "def foo():\n    # important comment\n    return 42\n";
    let mut rewriter = ByteRangeRewriter::new(source);
    // Replace 42 with 100
    let pos = source.find("42").expect("Should find 42");
    rewriter.add_edit(Edit::new(pos, pos + 2, "100"));

    let result = rewriter.apply().expect("should apply");
    assert!(result.contains("# important comment"));
    assert!(result.contains("return 100"));
}

#[test]
fn test_empty_edits() {
    let source = "hello world";
    let rewriter = ByteRangeRewriter::new(source);
    let result = rewriter.apply().expect("should apply");
    assert_eq!(result, source);
}

#[test]
fn test_adjacent_non_overlapping_edits() {
    let source = "abcdef";
    let mut rewriter = ByteRangeRewriter::new(source);
    // Replace "abc" and "def" adjacently
    rewriter.add_edit(Edit::new(0, 3, "XXX"));
    rewriter.add_edit(Edit::new(3, 6, "YYY"));

    let result = rewriter.apply().expect("should apply");
    assert_eq!(result, "XXXYYY");
}
