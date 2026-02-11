//! Pattern detection for scoring.

use super::ContextScorer;

impl ContextScorer {
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
