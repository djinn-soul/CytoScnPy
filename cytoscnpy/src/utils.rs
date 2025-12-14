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
    pub fn line_index(&self, offset: TextSize) -> usize {
        let offset = offset.to_usize();
        // Binary search to find which line range the offset falls into.
        match self.line_starts.binary_search(&offset) {
            Ok(line) => line + 1,
            Err(line) => line,
        }
    }
}

/// Detects lines with `# pragma: no cytoscnpy` comment.
///
/// Returns a set of line numbers (1-indexed) that should be ignored by the analyzer.
/// This allows users to suppress false positives or intentionally ignore specific lines.
pub fn get_ignored_lines(source: &str) -> FxHashSet<usize> {
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains("pragma: no cytoscnpy"))
        .map(|(i, _)| i + 1)
        .collect()
}

/// Checks if a path is a test path.
pub fn is_test_path(p: &str) -> bool {
    TEST_FILE_RE().is_match(p)
}

/// Checks if a path is a framework path.
pub fn is_framework_path(p: &str) -> bool {
    FRAMEWORK_FILE_RE().is_match(p)
}

/// Parses exclude folders, combining defaults with user inputs.
pub fn parse_exclude_folders(
    user_exclude_folders: Option<FxHashSet<String>>,
    use_defaults: bool,
    include_folders: Option<FxHashSet<String>>,
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
mod tests {
    use super::*;

    #[test]
    fn test_pragma_detection() {
        let source = r#"
def used_function():
    return 42

def unused_function():  # pragma: no cytoscnpy
    return "ignored"

class MyClass:  # pragma: no cytoscnpy
    pass
"#;
        let ignored = get_ignored_lines(source);

        // Lines 5 and 8 should be ignored (1-indexed)
        assert!(ignored.contains(&5), "Should detect pragma on line 5");
        assert!(ignored.contains(&8), "Should detect pragma on line 8");
        assert_eq!(ignored.len(), 2, "Should find exactly 2 pragma lines");
    }

    #[test]
    fn test_no_pragmas() {
        let source = r"
def regular_function():
    return 42
";
        let ignored = get_ignored_lines(source);
        assert_eq!(ignored.len(), 0, "Should find no pragma lines");
    }

    #[test]
    fn test_is_test_path() {
        assert!(is_test_path("tests/test_foo.py"));
        assert!(is_test_path("tests/foo_test.py"));
        assert!(is_test_path("project/tests/test_bar.py"));
        assert!(is_test_path("test_main.py"));
        assert!(is_test_path("my_test.py"));

        // Windows paths
        assert!(is_test_path("tests\\test_foo.py"));
        assert!(is_test_path("project\\tests\\test_bar.py"));

        // Negative cases
        assert!(!is_test_path("main.py"));
        assert!(!is_test_path("utils.py"));
        // "tests/utils.py" matches "tests/" prefix. So it IS a test path.
        assert!(is_test_path("tests/utils.py"));

        assert!(!is_test_path("prod_code.py"));
    }

    #[test]
    fn test_is_framework_path() {
        assert!(is_framework_path("views.py"));
        assert!(is_framework_path("api/views.py"));
        assert!(is_framework_path("handlers.py"));
        assert!(is_framework_path("routes.py"));
        assert!(is_framework_path("endpoints.py"));
        assert!(is_framework_path("api.py"));

        // Case insensitivity
        assert!(is_framework_path("Views.py"));

        // Negative cases
        assert!(!is_framework_path("main.py"));
        assert!(!is_framework_path("utils.py"));
        assert!(!is_framework_path("models.py")); // models.py is not in the default list
    }
}
