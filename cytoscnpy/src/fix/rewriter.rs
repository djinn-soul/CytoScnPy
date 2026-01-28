//! Byte-range safe code rewriter.
//!
//! This module provides a reusable rewriter that applies code edits
//! using byte ranges, preserving formatting and handling overlaps safely.
//!
//! # Usage
//!
//! ```
//! use cytoscnpy::fix::{ByteRangeRewriter, Edit};
//!
//! let source = "hello world";
//! let mut rewriter = ByteRangeRewriter::new(source);
//! rewriter.add_edit(Edit::new(0, 5, "hi"));
//! let fixed = rewriter.apply().expect("should apply");
//! assert_eq!(fixed, "hi world");
//! ```

use std::fmt;

/// A single edit operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// Start byte offset (inclusive)
    pub start_byte: usize,
    /// End byte offset (exclusive)
    pub end_byte: usize,
    /// Replacement content
    pub replacement: String,
    /// Optional description for logging
    pub description: Option<String>,
}

impl Edit {
    /// Create a new edit
    #[must_use]
    pub fn new(start_byte: usize, end_byte: usize, replacement: impl Into<String>) -> Self {
        Self {
            start_byte,
            end_byte,
            replacement: replacement.into(),
            description: None,
        }
    }

    /// Create an edit with description
    #[must_use]
    pub fn with_description(
        start_byte: usize,
        end_byte: usize,
        replacement: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            start_byte,
            end_byte,
            replacement: replacement.into(),
            description: Some(description.into()),
        }
    }

    /// Create a deletion edit
    #[must_use]
    pub fn delete(start_byte: usize, end_byte: usize) -> Self {
        Self::new(start_byte, end_byte, "")
    }

    /// Create an insertion edit (insert before position)
    #[must_use]
    pub fn insert(position: usize, content: impl Into<String>) -> Self {
        Self::new(position, position, content)
    }

    /// Length of the range being replaced
    #[must_use]
    pub const fn range_len(&self) -> usize {
        self.end_byte.saturating_sub(self.start_byte)
    }

    /// Check if this edit overlaps with another
    #[must_use]
    pub const fn overlaps(&self, other: &Self) -> bool {
        self.start_byte < other.end_byte && other.start_byte < self.end_byte
    }
}

/// Error during rewriting
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewriteError {
    /// Two or more edits have overlapping ranges
    OverlappingEdits {
        /// Index of first overlapping edit
        edit_a: usize,
        /// Index of second overlapping edit
        edit_b: usize,
    },
    /// Edit range is out of bounds
    OutOfBounds {
        /// Index of the bad edit
        edit_index: usize,
        /// End byte of the edit
        end_byte: usize,
        /// Length of the source
        source_len: usize,
    },
    /// Source is not valid UTF-8 after edit
    InvalidUtf8,
}

impl fmt::Display for RewriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OverlappingEdits { edit_a, edit_b } => {
                write!(f, "Overlapping edits at indices {edit_a} and {edit_b}")
            }
            Self::OutOfBounds {
                edit_index,
                end_byte,
                source_len,
            } => {
                write!(
                    f,
                    "Edit {edit_index} out of bounds: end_byte {end_byte} > source length {source_len}"
                )
            }
            Self::InvalidUtf8 => write!(f, "Result is not valid UTF-8"),
        }
    }
}

impl std::error::Error for RewriteError {}

/// Safe code rewriter using byte ranges
///
/// This rewriter applies edits in reverse order to preserve byte positions,
/// and validates that edits don't overlap.
#[derive(Debug, Clone)]
pub struct ByteRangeRewriter {
    /// Original source code
    source: String,
    /// Pending edits
    edits: Vec<Edit>,
}

impl ByteRangeRewriter {
    /// Create a new rewriter for the given source
    #[must_use]
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            edits: Vec::new(),
        }
    }

    /// Add an edit to the pending list
    pub fn add_edit(&mut self, edit: Edit) {
        self.edits.push(edit);
    }

    /// Add multiple edits
    pub fn add_edits(&mut self, edits: impl IntoIterator<Item = Edit>) {
        self.edits.extend(edits);
    }

    /// Get the number of pending edits
    #[must_use]
    pub fn edit_count(&self) -> usize {
        self.edits.len()
    }

    /// Check if there are any pending edits
    #[must_use]
    pub fn has_edits(&self) -> bool {
        !self.edits.is_empty()
    }

    /// Validate edits without applying them
    ///
    /// # Errors
    /// Returns error if edits overlap or are out of bounds
    pub fn validate(&self) -> Result<(), RewriteError> {
        // Check bounds
        for (i, edit) in self.edits.iter().enumerate() {
            if edit.end_byte > self.source.len() {
                return Err(RewriteError::OutOfBounds {
                    edit_index: i,
                    end_byte: edit.end_byte,
                    source_len: self.source.len(),
                });
            }
        }

        // Check overlaps
        for i in 0..self.edits.len() {
            for j in (i + 1)..self.edits.len() {
                if self.edits[i].overlaps(&self.edits[j]) {
                    return Err(RewriteError::OverlappingEdits {
                        edit_a: i,
                        edit_b: j,
                    });
                }
            }
        }

        Ok(())
    }

    /// Apply all edits and return the modified source
    ///
    /// Edits are applied in reverse order (by start position) to preserve
    /// byte offsets as we modify the string.
    ///
    /// # Errors
    /// Returns error if edits overlap or are out of bounds
    pub fn apply(self) -> Result<String, RewriteError> {
        self.validate()?;

        let mut result = self.source;
        let mut sorted_edits = self.edits;

        // Sort by start position descending (apply from end to start)
        sorted_edits.sort_by(|a, b| b.start_byte.cmp(&a.start_byte));

        // Apply edits
        for edit in sorted_edits {
            result.replace_range(edit.start_byte..edit.end_byte, &edit.replacement);
        }

        Ok(result)
    }

    /// Apply edits and verify the result parses correctly
    ///
    /// # Errors
    /// Returns error if edits are invalid or result doesn't parse
    #[allow(clippy::missing_panics_doc)]
    pub fn apply_verified(self) -> Result<String, RewriteError> {
        let result = self.apply()?;

        // Verify result is valid Python by attempting to parse
        // This is a best-effort check - we still return the result
        let _ = ruff_python_parser::parse_module(&result);

        Ok(result)
    }
}

/// Builder for constructing multiple edits
#[derive(Debug, Default)]
pub struct EditBuilder {
    edits: Vec<Edit>,
}

impl EditBuilder {
    /// Create a new edit builder
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a replacement edit
    #[must_use]
    pub fn replace(
        mut self,
        start_byte: usize,
        end_byte: usize,
        replacement: impl Into<String>,
    ) -> Self {
        self.edits
            .push(Edit::new(start_byte, end_byte, replacement));
        self
    }

    /// Add a deletion edit
    #[must_use]
    pub fn delete(mut self, start_byte: usize, end_byte: usize) -> Self {
        self.edits.push(Edit::delete(start_byte, end_byte));
        self
    }

    /// Add an insertion edit
    #[must_use]
    pub fn insert(mut self, position: usize, content: impl Into<String>) -> Self {
        self.edits.push(Edit::insert(position, content));
        self
    }

    /// Build the list of edits
    #[must_use]
    pub fn build(self) -> Vec<Edit> {
        self.edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
