//! Context-based scoring engine for secret findings.
//!
//! This module provides confidence scoring based on contextual signals
//! like file type, proximity to keywords, entropy, etc.

use std::path::Path;

/// Scoring context for evaluating a finding.
pub struct ScoringContext<'a> {
    /// The content of the line where the finding was detected.
    pub line_content: &'a str,
    /// Path to the file being analyzed.
    pub file_path: &'a Path,
    /// Whether the finding is in a comment.
    pub is_comment: bool,
    /// Whether the finding is in a docstring.
    pub is_docstring: bool,
}

/// Score adjustments for various contextual signals.
#[derive(Debug, Clone)]
pub struct ScoringAdjustments {
    /// Bonus when near a suspicious keyword (`api_key=`).
    pub near_keyword: i16,
    /// Penalty for findings in test files.
    pub in_test_file: i16,
    /// Penalty for findings in comments.
    pub in_comment: i16,
    /// Bonus for high entropy strings.
    pub high_entropy: i16,
    /// Penalty for `os.environ.get()` patterns.
    pub is_env_var: i16,
    /// Penalty for paths/URLs.
    pub is_path_or_url: i16,
    /// Penalty for pragma: no cytoscnpy.
    pub has_pragma: i16,
    /// Penalty for placeholder values.
    pub is_placeholder: i16,
}

impl Default for ScoringAdjustments {
    fn default() -> Self {
        Self {
            near_keyword: 20,
            in_test_file: -50,
            in_comment: -10,
            high_entropy: 15,
            is_env_var: -100,
            is_path_or_url: -100,
            has_pragma: -100,
            is_placeholder: -30,
        }
    }
}

/// Context-based scorer that adjusts confidence based on signals.
pub struct ContextScorer {
    adjustments: ScoringAdjustments,
}

impl Default for ContextScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextScorer {
    /// Creates a new context scorer with default adjustments.
    #[must_use]
    pub fn new() -> Self {
        Self {
            adjustments: ScoringAdjustments::default(),
        }
    }

    /// Creates a new context scorer with custom adjustments.
    #[must_use]
    pub fn with_adjustments(adjustments: ScoringAdjustments) -> Self {
        Self { adjustments }
    }

    /// Apply scoring adjustments to a base score based on context.
    ///
    /// Returns the adjusted score, clamped to 0-100.
    #[must_use]
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn score(&self, base_score: u8, ctx: &ScoringContext<'_>) -> u8 {
        let mut score = i16::from(base_score);

        // Check if in test file
        if self.is_test_file(ctx.file_path) {
            score += self.adjustments.in_test_file;
        }

        // Check if in comment
        if ctx.is_comment {
            score += self.adjustments.in_comment;
        }

        // Check if in docstring
        if ctx.is_docstring {
            score += self.adjustments.in_comment; // Same penalty as comments
        }

        // Check for pragma
        if ctx.line_content.contains("pragma: no cytoscnpy") {
            score += self.adjustments.has_pragma;
        }

        // Check for environment variable patterns
        if self.is_env_var_access(ctx.line_content) {
            score += self.adjustments.is_env_var;
        }

        // Check for path/URL patterns
        if self.looks_like_path_or_url(ctx.line_content) {
            score += self.adjustments.is_path_or_url;
        }

        // Check for placeholder patterns
        if self.is_placeholder(ctx.line_content) {
            score += self.adjustments.is_placeholder;
        }

        // Clamp to 0-100
        score.clamp(0, 100) as u8
    }

    /// Bonus score for high entropy strings.
    #[must_use]
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn entropy_bonus(&self, base_score: u8) -> u8 {
        let score = i16::from(base_score) + self.adjustments.high_entropy;
        score.clamp(0, 100) as u8
    }

    /// Bonus score for being near a suspicious keyword.
    #[must_use]
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn keyword_bonus(&self, base_score: u8) -> u8 {
        let score = i16::from(base_score) + self.adjustments.near_keyword;
        score.clamp(0, 100) as u8
    }

    /// Checks if the file is a test file.
    #[allow(clippy::unused_self)]
    fn is_test_file(&self, path: &Path) -> bool {
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
    fn is_env_var_access(&self, line: &str) -> bool {
        let lower = line.to_lowercase();
        lower.contains("os.environ")
            || lower.contains("os.getenv")
            || lower.contains("environ.get")
            || lower.contains("environ[")
    }

    /// Checks if a string looks like a file path or URL.
    #[allow(clippy::unused_self)]
    fn looks_like_path_or_url(&self, s: &str) -> bool {
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
    fn is_placeholder(&self, line: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_scorer_test_file_detection() {
        let scorer = ContextScorer::new();

        assert!(scorer.is_test_file(Path::new("/project/tests/test_secrets.py")));
        assert!(scorer.is_test_file(Path::new("/project/test/test_main.py")));
        assert!(scorer.is_test_file(Path::new("/project/src/test_utils.py")));
        assert!(scorer.is_test_file(Path::new("/project/conftest.py")));
        assert!(!scorer.is_test_file(Path::new("/project/src/main.py")));
    }

    #[test]
    fn test_scorer_env_var_detection() {
        let scorer = ContextScorer::new();

        assert!(scorer.is_env_var_access("password = os.environ.get('PASSWORD')"));
        assert!(scorer.is_env_var_access("key = os.getenv('API_KEY')"));
        assert!(!scorer.is_env_var_access("password = 'hardcoded'"));
    }

    #[test]
    fn test_scorer_placeholder_detection() {
        let scorer = ContextScorer::new();

        assert!(scorer.is_placeholder("api_key = 'xxx123'"));
        assert!(scorer.is_placeholder("secret = 'your_secret_here'"));
        assert!(scorer.is_placeholder("token = '${TOKEN}'"));
        assert!(!scorer.is_placeholder("api_key = 'sk_live_abc123'"));
    }

    #[test]
    fn test_scorer_scoring() {
        let scorer = ContextScorer::new();
        let path = PathBuf::from("/project/src/main.py");

        let ctx = ScoringContext {
            line_content: "password = 'secret123'",
            file_path: &path,
            is_comment: false,
            is_docstring: false,
        };

        // Base score should remain unchanged for normal context
        assert_eq!(scorer.score(70, &ctx), 70);

        // Test file should reduce score
        let test_path = PathBuf::from("/project/tests/test_main.py");
        let test_ctx = ScoringContext {
            line_content: "password = 'secret123'",
            file_path: &test_path,
            is_comment: false,
            is_docstring: false,
        };
        assert_eq!(scorer.score(70, &test_ctx), 20); // 70 - 50 = 20

        // Env var should reduce score to 0
        let env_ctx = ScoringContext {
            line_content: "password = os.environ.get('PASSWORD')",
            file_path: &path,
            is_comment: false,
            is_docstring: false,
        };
        assert_eq!(scorer.score(70, &env_ctx), 0); // 70 - 100, clamped to 0
    }
}
