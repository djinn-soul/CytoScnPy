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
use rustc_hash::{FxHashMap, FxHashSet};

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

/// Suppression specification.
#[derive(Debug, Clone, PartialEq)]
pub enum Suppression {
    /// Suppress all findings.
    All,
    /// Suppress findings for specific rule IDs.
    Specific(FxHashSet<String>),
}

impl Suppression {
    /// Returns whether this suppression covers the requested rule ID.
    #[must_use]
    pub fn covers(&self, rule_id: &str) -> bool {
        match self {
            Self::All => true,
            Self::Specific(rules) => rules.contains(&rule_id.to_uppercase()),
        }
    }
}

/// Detects suppression specification for a line.
///
/// Supports multiple formats:
/// - `# pragma: no cytoscnpy` - Legacy format (All)
/// - `# noqa` or `# ignore` - Bare ignore (All)
/// - `# noqa: CSP-D101, CSP-Q202` - Specific codes
#[must_use]
pub fn get_line_suppression(line: &str) -> Option<Suppression> {
    let re = crate::constants::SUPPRESSION_RE();

    if let Some(caps) = re.captures(line) {
        // Case 1: # pragma: no cytoscnpy -> Always ignore
        if line.to_lowercase().contains("pragma: no cytoscnpy") {
            return Some(Suppression::All);
        }

        // Case 2: Specific codes
        if let Some(codes_match) = caps.get(1) {
            let codes_str = codes_match.as_str();
            let mut specific_rules = FxHashSet::default();
            for code in codes_str.split(',').map(str::trim) {
                let c = code.to_uppercase();
                if c == "CSP" {
                    return Some(Suppression::All); // Treat generic "CSP" as suppress all
                }
                specific_rules.insert(c);
            }
            if !specific_rules.is_empty() {
                return Some(Suppression::Specific(specific_rules));
            }
            // If codes exist but none are CSP-related, we don't suppress CSP findings
            return None;
        }

        // Case 3: Bare ignore (no colon/codes) -> Always ignore
        return Some(Suppression::All);
    }

    None
}

/// Detects lines with suppression comments in a source file.
///
/// Returns a map of line numbers (1-indexed) to suppression specs.
#[must_use]
pub fn get_ignored_lines(source: &str) -> FxHashMap<usize, Suppression> {
    source
        .lines()
        .enumerate()
        .filter_map(|(i, line)| get_line_suppression(line).map(|suppression| (i + 1, suppression)))
        .collect()
}

/// Checks if a path is a test path.
#[must_use]
pub fn is_test_path(p: &str) -> bool {
    TEST_FILE_RE().is_match(p)
}

/// Checks whether a path is a test path relative to an analysis root.
#[must_use]
pub fn is_test_path_relative_to(path: &std::path::Path, root: &std::path::Path) -> bool {
    let relative = if root.is_file() {
        path.file_name().map(std::path::Path::new).unwrap_or(path)
    } else {
        path.strip_prefix(root).unwrap_or(path)
    };
    is_test_path(&relative.to_string_lossy())
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

/// Checks if a specific line and rule are suppressed.
///
/// Returns true if the finding should be ignored.
#[must_use]
pub fn is_line_suppressed(
    ignored_lines: &std::collections::HashMap<usize, Suppression, impl std::hash::BuildHasher>,
    line: usize,
    rule_id: &str,
) -> bool {
    ignored_lines
        .get(&line)
        .is_some_and(|suppression| suppression.covers(rule_id))
}

/// Returns whether an inline comment suppresses the requested rule ID.
#[must_use]
pub fn is_rule_suppressed_on_line(line: &str, rule_id: &str) -> bool {
    get_line_suppression(line).is_some_and(|suppression| suppression.covers(rule_id))
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
