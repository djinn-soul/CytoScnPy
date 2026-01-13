//! Utilities module for CytoScnPy.
//!
//! This module provides various utility functions used across the codebase.

mod paths;

// Re-export path utilities for backward compatibility
pub use paths::{
    collect_python_files_gitignore, is_excluded, normalize_display_path, validate_output_path,
    validate_path_within_root,
};

use crate::constants::{DEFAULT_EXCLUDE_FOLDERS, FRAMEWORK_FILE_RE, TEST_FILE_RE};
use ruff_text_size::TextSize;
use rustc_hash::FxHashSet;

/// A utility struct to convert byte offsets to line numbers.
///
/// This is necessary because the AST parser works with byte offsets,
/// but we want to report findings with line numbers which are more human-readable.
#[derive(Debug, Clone)]
pub struct LineIndex {
    /// Stores the byte index of the start of each line.
    line_starts: Vec<usize>,
}

impl LineIndex {
    /// Creates a new `LineIndex` by scanning the source code for newlines.
    /// Uses byte iteration for performance since '\n' is always a single byte in UTF-8.
    #[must_use]
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        // Use bytes() instead of char_indices() - newlines are always single bytes in UTF-8
        for (i, byte) in source.as_bytes().iter().enumerate() {
            if *byte == b'\n' {
                // Record the start of the next line (current newline index + 1)
                line_starts.push(i + 1);
            }
        }
        Self { line_starts }
    }

    /// Converts a `TextSize` (byte offset) to a 1-indexed line number.
    #[must_use]
    pub fn line_index(&self, offset: TextSize) -> usize {
        let offset = offset.to_usize();
        // Binary search to find which line range the offset falls into.
        match self.line_starts.binary_search(&offset) {
            Ok(line) => line + 1,
            Err(line) => line,
        }
    }
}

/// Detects if a line should be ignored based on suppression comments.
///
/// Supports multiple formats:
/// - `# pragma: no cytoscnpy` - Legacy format
/// - `# noqa` or `# ignore` - Bare ignore (ignores all)
/// - `# noqa: CSP, E501` - Specific codes (ignores if CSP is present)
#[must_use]
pub fn is_line_suppressed(line: &str) -> bool {
    let re = crate::constants::SUPPRESSION_RE();

    if let Some(caps) = re.captures(line) {
        // Case 1: # pragma: no cytoscnpy -> Always ignore
        if line.to_lowercase().contains("pragma: no cytoscnpy") {
            return true;
        }

        // Case 2: Specific codes
        if let Some(codes_match) = caps.get(1) {
            let codes_str = codes_match.as_str();
            // If it's something like # noqa: E501, we only ignore if CSP is in the list
            return codes_str.split(',').map(str::trim).any(|code| {
                let c = code.to_uppercase();
                c == "CSP" || c.starts_with("CSP")
            });
        }

        // Case 3: Bare ignore (no colon/codes) -> Always ignore
        return true;
    }

    false
}

/// Detects lines with suppression comments in a source file.
///
/// Returns a set of line numbers (1-indexed) that should be ignored by the analyzer.
#[must_use]
pub fn get_ignored_lines(source: &str) -> FxHashSet<usize> {
    source
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            if is_line_suppressed(line) {
                Some(i + 1)
            } else {
                None
            }
        })
        .collect()
}

/// Checks if a path is a test path.
#[must_use]
pub fn is_test_path(p: &str) -> bool {
    TEST_FILE_RE().is_match(p)
}

/// Checks if a path is a framework path.
#[must_use]
pub fn is_framework_path(p: &str) -> bool {
    FRAMEWORK_FILE_RE().is_match(p)
}

/// Parses exclude folders, combining defaults with user inputs.
pub fn parse_exclude_folders<S: std::hash::BuildHasher>(
    user_exclude_folders: Option<std::collections::HashSet<String, S>>,
    use_defaults: bool,
    include_folders: Option<std::collections::HashSet<String, S>>,
) -> FxHashSet<String> {
    let mut exclude_folders = FxHashSet::default();

    if use_defaults {
        for folder in DEFAULT_EXCLUDE_FOLDERS() {
            exclude_folders.insert((*folder).to_owned());
        }
    }

    if let Some(user_folders) = user_exclude_folders {
        exclude_folders.extend(user_folders);
    }

    if let Some(include) = include_folders {
        for folder in include {
            exclude_folders.remove(&folder);
        }
    }

    exclude_folders
}
