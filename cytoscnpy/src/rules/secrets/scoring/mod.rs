//! Context-based scoring engine for secret findings.
//!
//! This module provides confidence scoring based on contextual signals
//! like file type, proximity to keywords, entropy, etc.

mod detection;
mod tests;

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

        // Check for suppression comments
        if crate::utils::get_line_suppression(ctx.line_content).is_some() {
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
}
