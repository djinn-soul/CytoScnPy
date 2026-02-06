use crate::utils::LineIndex;
use ruff_python_ast::Stmt;
use std::path::PathBuf;

/// A raw finding before scoring is applied.
#[derive(Debug, Clone)]
pub struct RawFinding {
    /// Description of the finding.
    pub message: String,
    /// Unique rule identifier (e.g., "CSP-S101").
    pub rule_id: String,
    /// Line number (1-indexed).
    pub line: usize,
    /// Base confidence score (0-100) for this finding.
    pub base_score: u8,
    /// The matched value (redacted for security).
    pub matched_value: Option<String>,
    /// Entropy score (if applicable).
    pub entropy: Option<f64>,
    /// Severity level.
    pub severity: String,
}

/// Trait for pluggable secret recognizers.
///
/// Recognizers can scan text content and/or AST nodes to detect secrets.
pub trait SecretRecognizer: Send + Sync {
    /// Name of the recognizer for logging/debugging.
    fn name(&self) -> &'static str;

    /// Base confidence score (0-100) for findings from this recognizer.
    fn base_score(&self) -> u8;

    /// Scan text content for secrets. Returns raw findings before scoring.
    fn scan_text(&self, content: &str, file_path: &PathBuf) -> Vec<RawFinding>;

    /// Scan text content when AST is unavailable (default: same as `scan_text`).
    fn scan_text_fallback(&self, content: &str, file_path: &PathBuf) -> Vec<RawFinding> {
        self.scan_text(content, file_path)
    }

    /// Scan AST for secrets (optional, default returns empty).
    fn scan_ast(
        &self,
        _stmts: &[Stmt],
        _file_path: &PathBuf,
        _line_index: &LineIndex,
    ) -> Vec<RawFinding> {
        Vec::new()
    }
}

pub(super) fn is_test_name(lower: &str) -> bool {
    lower == "test"
        || lower.starts_with("test_")
        || lower.ends_with("_test")
        || lower.contains("_test_")
}
