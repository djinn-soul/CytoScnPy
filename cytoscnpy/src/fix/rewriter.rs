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

use super::edit::{Edit, RewriteError};

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
            if edit.start_byte > edit.end_byte {
                return Err(RewriteError::InvalidRange {
                    edit_index: i,
                    start_byte: edit.start_byte,
                    end_byte: edit.end_byte,
                });
            }
            if edit.end_byte > self.source.len() {
                return Err(RewriteError::OutOfBounds {
                    edit_index: i,
                    end_byte: edit.end_byte,
                    source_len: self.source.len(),
                });
            }
            if !self.source.is_char_boundary(edit.start_byte)
                || !self.source.is_char_boundary(edit.end_byte)
            {
                return Err(RewriteError::InvalidUtf8);
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
        sorted_edits.sort_by_key(|e| std::cmp::Reverse(e.start_byte));

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
    pub fn apply_verified(self) -> Result<String, RewriteError> {
        let result = self.apply()?;

        ruff_python_parser::parse_module(&result).map_err(|error| RewriteError::InvalidPython {
            error: error.to_string(),
        })?;

        Ok(result)
    }
}

#[cfg(test)]
#[path = "rewriter_tests.rs"]
mod tests;
