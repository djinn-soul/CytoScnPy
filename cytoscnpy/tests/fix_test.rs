//! Tests for the fix module (ByteRangeRewriter).

use cytoscnpy::fix::{ByteRangeRewriter, Edit, EditBuilder, RewriteError};

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
    let mut rewriter = ByteRangeRewriter::new(source);
    let start = source.find("def unused_func").unwrap();
    let end = source.find("def another_used").unwrap();
    rewriter.add_edit(Edit::delete(start, end));

    let result = rewriter.apply().expect("should apply");
    assert!(result.contains("def used_func"));
    assert!(!result.contains("def unused_func"));
    assert!(result.contains("def another_used"));
}

#[test]
fn test_python_import_deletion() {
    let source = r#"import os
import sys
from typing import List

def main():
    print(sys.version)
"#;
    let mut rewriter = ByteRangeRewriter::new(source);
    let end = source.find("import sys").unwrap();
    rewriter.add_edit(Edit::delete(0, end));

    let result = rewriter.apply().expect("should apply");
    assert!(!result.contains("import os"));
    assert!(result.contains("import sys"));
}

#[test]
fn test_multiple_deletions_same_file() {
    let source = r#"import os
import sys
import json

def func_a():
    pass

def func_b():
    pass

def func_c():
    pass
"#;
    let mut rewriter = ByteRangeRewriter::new(source);

    let os_end = source.find("import sys").unwrap();
    rewriter.add_edit(Edit::delete(0, os_end));

    let func_b_start = source.find("def func_b").unwrap();
    let func_b_end = source.find("def func_c").unwrap();
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
    let pos = source.find("42").unwrap();
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
    rewriter.add_edit(Edit::new(0, 3, "XXX"));
    rewriter.add_edit(Edit::new(3, 6, "YYY"));

    let result = rewriter.apply().expect("should apply");
    assert_eq!(result, "XXXYYY");
}

#[test]
fn test_python_class_deletion() {
    let source = r#"class UsedClass:
    def method(self):
        return "used"

class UnusedClass:
    def method(self):
        return "unused"

obj = UsedClass()
"#;
    let mut rewriter = ByteRangeRewriter::new(source);
    let start = source.find("class UnusedClass").unwrap();
    let end = source.find("obj = UsedClass").unwrap();
    rewriter.add_edit(Edit::delete(start, end));

    let result = rewriter.apply().expect("should apply");
    assert!(result.contains("class UsedClass"));
    assert!(!result.contains("class UnusedClass"));
    assert!(result.contains("obj = UsedClass"));
}

#[test]
fn test_edit_with_description() {
    let edit = Edit::with_description(0, 5, "hi", "Replace hello with hi");
    assert_eq!(edit.start_byte, 0);
    assert_eq!(edit.end_byte, 5);
    assert_eq!(edit.replacement, "hi");
    assert_eq!(edit.description, Some("Replace hello with hi".to_owned()));
}

#[test]
fn test_edit_range_len() {
    let edit = Edit::new(5, 15, "replacement");
    assert_eq!(edit.range_len(), 10);
}

#[test]
fn test_rewriter_has_edits() {
    let mut rewriter = ByteRangeRewriter::new("test");
    assert!(!rewriter.has_edits());

    rewriter.add_edit(Edit::new(0, 1, "x"));
    assert!(rewriter.has_edits());
}

#[test]
fn test_rewriter_edit_count() {
    let mut rewriter = ByteRangeRewriter::new("test");
    assert_eq!(rewriter.edit_count(), 0);

    rewriter.add_edit(Edit::new(0, 1, "x"));
    rewriter.add_edit(Edit::new(2, 3, "y"));
    assert_eq!(rewriter.edit_count(), 2);
}

#[test]
fn test_add_multiple_edits() {
    let mut rewriter = ByteRangeRewriter::new("abc123xyz");
    let edits = vec![Edit::new(0, 3, "AAA"), Edit::new(6, 9, "ZZZ")];
    rewriter.add_edits(edits);

    let result = rewriter.apply().expect("should apply");
    assert_eq!(result, "AAA123ZZZ");
}
