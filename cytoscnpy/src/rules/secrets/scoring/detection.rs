//! Pattern detection for scoring.

use super::ContextScorer;
use std::path::Path;

impl ContextScorer {
    /// Checks if the file is a test file.
    #[allow(clippy::unused_self)]
    pub(crate) fn is_test_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();

        // Check for common test directory patterns
        if path_str.contains("/test/")
            || path_str.contains("/tests/")
            || path_str.contains("\\test\\")
            || path_str.contains("\\tests\\")
        {
            return true;
        }

        // Check for test file naming patterns
        if let Some(file_name) = path.file_name() {
            let name = file_name.to_string_lossy().to_lowercase();
            if name.starts_with("test_")
                || name.ends_with("_test.py")
                || name.ends_with("_tests.py")
                || name == "conftest.py"
            {
                return true;
            }
        }

        false
    }

    /// Checks if the line contains an environment variable access pattern.
    #[allow(clippy::unused_self)]
    pub(crate) fn is_env_var_access(&self, line: &str) -> bool {
        let lower = line.to_lowercase();
        lower.contains("os.environ")
            || lower.contains("os.getenv")
            || lower.contains("environ.get")
            || lower.contains("environ[")
    }

    /// Checks if a string looks like a file path or URL.
    #[allow(clippy::unused_self)]
    pub(crate) fn looks_like_path_or_url(&self, s: &str) -> bool {
        // URL patterns
        if s.contains("http://") || s.contains("https://") || s.contains("ftp://") {
            return true;
        }
        // File path patterns: check for path-like structures in quotes
        if s.contains("\"/") || s.contains("\"./") || s.contains("\"~/") {
            return true;
        }
        if s.contains("'\\") || s.contains("\"\\") {
            return true;
        }
        false
    }

    /// Checks if the value looks like a placeholder.
    #[allow(clippy::unused_self)]
    pub(crate) fn is_placeholder(&self, line: &str) -> bool {
        let lower = line.to_lowercase();
        // Common placeholder patterns
        lower.contains("\"xxx")
            || lower.contains("'xxx")
            || lower.contains("\"your_")
            || lower.contains("'your_")
            || lower.contains("\"changeme")
            || lower.contains("'changeme")
            || lower.contains("\"replace_")
            || lower.contains("'replace_")
            || lower.contains("\"example")
            || lower.contains("'example")
            || lower.contains("<your_")
            || lower.contains("${")
            || lower.contains("{{")
    }
}
