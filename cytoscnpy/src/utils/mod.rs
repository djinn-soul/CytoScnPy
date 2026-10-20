//! Utilities module for CytoScnPy.
//!
//! This module provides various utility functions used across the codebase.

mod ignore_match;
mod minified;
pub(crate) mod path_inventory;
mod paths;
pub(crate) mod python_guards;
pub(crate) mod sources;
mod suppression;

pub(crate) use paths::discover_project_root;

// Re-export path, ignore, and minification utilities
pub use ignore_match::{is_name_or_pattern_ignored, is_path_ignored};
pub(crate) use minified::retain_unminified;
pub use minified::{is_likely_minified, is_likely_minified_content, is_likely_minified_path};
pub(crate) use paths::collect_python_files_within;
pub use paths::{
    collect_python_files_gitignore, collect_python_files_gitignore_with_errors, is_excluded,
    normalize_display_path, validate_output_path, validate_path_within_root,
};
pub use suppression::{
    get_ignored_lines, get_line_suppression, is_line_suppressed, is_rule_suppressed_on_line,
    Suppression,
};

use crate::constants::{DEFAULT_EXCLUDE_FOLDERS, FRAMEWORK_FILE_RE, TEST_FILE_RE};
use ruff_text_size::TextSize;
use rustc_hash::FxHashSet;

/// Frees `value` on the rayon pool instead of the calling thread.
///
/// Large scans build millions of small allocations; freeing them serially
/// takes a noticeable share of the run while nothing waits on the result.
pub(crate) fn drop_in_background<T: Send + 'static>(value: T) {
    rayon::spawn(move || drop(value));
}

/// A utility struct to convert byte offsets to line numbers.
///
/// This is necessary because the AST parser works with byte offsets,
/// but we want to report findings with line numbers which are more human-readable.
#[derive(Debug, Clone)]
pub struct LineIndex {
    /// Source text, retained only for non-ASCII files, where a byte offset and a
    /// character column diverge. `None` for ASCII sources, so the common case does
    /// not pay for a second copy of the file.
    source: Option<String>,
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
        Self {
            source: (!source.is_ascii()).then(|| source.to_owned()),
            line_starts,
        }
    }

    /// Converts a `TextSize` (byte offset) to a 1-indexed line number.
    #[must_use]
    pub fn line_index(&self, offset: TextSize) -> usize {
        let offset = offset.to_usize();
        match self.line_starts.binary_search(&offset) {
            Ok(line) => line + 1,
            Err(line) => line,
        }
    }

    /// Converts a `TextSize` (byte offset) to a 1-indexed character column.
    #[must_use]
    pub fn column_index(&self, offset: TextSize) -> usize {
        let offset_usize = offset.to_usize();
        let line = self.line_index(offset);
        if line == 0 {
            return 1;
        }
        let line_start = self.line_starts[line - 1];
        match &self.source {
            Some(source) => source[line_start..offset_usize].chars().count() + 1,
            None => offset_usize - line_start + 1,
        }
    }
}

/// Checks if a path is a test path.
#[must_use]
pub fn is_test_path(p: &str) -> bool {
    TEST_FILE_RE().is_match(p)
}

/// Checks whether a path is a test path relative to an analysis root.
/// File roots discover their nearest project boundary; directory roots are
/// authoritative. Paths outside the root use only their filename, never test
/// directory names from unrelated absolute ancestors.
#[must_use]
pub fn is_test_path_relative_to(path: &std::path::Path, root: &std::path::Path) -> bool {
    let discovered_root;
    let root = if root.is_file() {
        discovered_root = discover_project_root(root.parent().unwrap_or(std::path::Path::new(".")));
        discovered_root.as_path()
    } else {
        root
    };
    let relative = path
        .strip_prefix(root)
        .ok()
        .filter(|relative| !relative.as_os_str().is_empty())
        .unwrap_or_else(|| path.file_name().map(std::path::Path::new).unwrap_or(path));
    is_test_path(&relative.to_string_lossy())
}

#[cfg(test)]
mod test_path_tests {
    use super::is_test_path_relative_to;
    use std::path::Path;

    #[test]
    fn ignores_test_segments_above_the_analysis_root() {
        let root = Path::new("/workspace/tests/project");

        assert!(!is_test_path_relative_to(
            Path::new("/workspace/tests/project/src/main.py"),
            root,
        ));
        assert!(is_test_path_relative_to(
            Path::new("/workspace/tests/project/tests/test_main.py"),
            root,
        ));
    }
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

#[cfg(test)]
mod line_index_tests {
    use super::LineIndex;
    use ruff_text_size::TextSize;

    #[test]
    fn column_index_counts_characters_not_bytes() {
        let ascii = "x = 1\nimport os\n";
        let index = LineIndex::new(ascii);
        let offset = TextSize::from(u32::try_from(ascii.find("os").unwrap()).unwrap());
        assert_eq!(index.line_index(offset), 2);
        assert_eq!(index.column_index(offset), 8);

        // 'é' is two bytes in UTF-8, so a byte column would report 13 here.
        let unicode = "x = 1\nrésultat = os\n";
        let index = LineIndex::new(unicode);
        let offset = TextSize::from(u32::try_from(unicode.find("os").unwrap()).unwrap());
        assert_eq!(index.line_index(offset), 2);
        assert_eq!(index.column_index(offset), 12);
    }
}
