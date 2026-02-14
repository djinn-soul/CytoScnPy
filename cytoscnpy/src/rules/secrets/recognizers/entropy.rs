use super::types::{RawFinding, SecretRecognizer};
use crate::utils::LineIndex;
use ruff_python_ast::Stmt;
use std::collections::HashMap;
use std::path::PathBuf;

/// High-entropy string detection recognizer.
pub struct EntropyRecognizer {
    /// Minimum entropy threshold.
    pub threshold: f64,
    /// Minimum string length to check.
    pub min_length: usize,
}

impl Default for EntropyRecognizer {
    fn default() -> Self {
        Self {
            threshold: 4.5,
            min_length: 16,
        }
    }
}

impl EntropyRecognizer {
    /// Creates a new entropy recognizer with the given threshold and min length.
    #[must_use]
    pub fn new(threshold: f64, min_length: usize) -> Self {
        Self {
            threshold,
            min_length,
        }
    }

    /// Calculate Shannon entropy of a string.
    #[allow(clippy::cast_precision_loss)]
    fn calculate_entropy(s: &str) -> f64 {
        if s.is_empty() {
            return 0.0;
        }

        let mut char_counts: HashMap<char, usize> = HashMap::new();
        let len = s.len() as f64;

        for c in s.chars() {
            *char_counts.entry(c).or_insert(0) += 1;
        }

        char_counts
            .values()
            .map(|&count| {
                let p = count as f64 / len;
                -p * p.log2()
            })
            .sum()
    }

    /// Extract quoted strings from a line.
    pub(super) fn extract_string_literals(line: &str) -> Vec<&str> {
        let mut strings = Vec::new();
        let mut in_string = false;
        let mut quote_char = ' ';
        let mut start = 0;
        let mut escaped = false;

        for (i, c) in line.char_indices() {
            if !in_string && (c == '"' || c == '\'') {
                in_string = true;
                quote_char = c;
                start = i + 1;
                escaped = false;
            } else if in_string {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == quote_char {
                    if i > start {
                        strings.push(&line[start..i]);
                    }
                    in_string = false;
                }
            }
        }

        strings
    }

    /// Check if a string looks like a path or URL.
    fn looks_like_path_or_url(s: &str) -> bool {
        if s.starts_with("data:") {
            return true;
        }
        if s.starts_with("http://") || s.starts_with("https://") || s.starts_with("ftp://") {
            return true;
        }
        if s.contains('/') && (s.starts_with('/') || s.starts_with('.') || s.starts_with('~')) {
            return true;
        }
        if s.contains('\\') && (s.len() > 2 && s.chars().nth(1) == Some(':')) {
            return true;
        }
        // Package paths like "com.example.package"
        if s.chars().filter(|&c| c == '.').count() >= 2 && !s.contains(' ') {
            return true;
        }
        false
    }

    /// Redact a secret value (show first 4 and last 4 chars).
    fn redact_value(s: &str) -> String {
        if s.len() <= 8 {
            return "*".repeat(s.len());
        }
        let start: String = s.chars().take(4).collect();
        let end: String = s
            .chars()
            .rev()
            .take(4)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        format!("{start}...{end}")
    }

    pub(super) fn check_string(&self, s: &str, line: usize, findings: &mut Vec<RawFinding>) {
        if s.len() >= self.min_length {
            // Optimization: High-entropy secrets (API keys, tokens, hashes)
            // almost never contain many spaces. Natural language, SQL, and
            // other structured data that often trigger false positives do.
            if s.chars().filter(|&c| c == ' ').count() >= 3 {
                return;
            }

            // Skip common high-entropy data blobs that aren't usually secrets
            // 1. Base64 padding or length suggesting a blob
            if s.len() > 64 && (s.ends_with('=') || s.chars().any(|c| c == '+' || c == '/')) {
                // Check if it strictly follows Base64 charset
                if s.chars()
                    .all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '=')
                {
                    return;
                }
            }
            // 2. Very long hex strings (often binary data or hashes we don't care about here)
            if s.len() > 128 && s.chars().all(|c| c.is_ascii_hexdigit()) {
                return;
            }
            // 3. UUID-like structures (e.g., f47ac10b-58cc-4372-a567-0e02b2c3d479)
            if s.len() == 36 && s.chars().filter(|&c| c == '-').count() == 4 {
                // Quick check for hex segments between hyphens
                return;
            }

            let entropy = Self::calculate_entropy(s);
            if entropy >= self.threshold && !Self::looks_like_path_or_url(s) {
                findings.push(RawFinding {
                    message: format!("High-entropy string detected (entropy: {entropy:.2})"),
                    rule_id: "CSP-S200".to_owned(),
                    line,
                    base_score: self.base_score(),
                    matched_value: Some(Self::redact_value(s)),
                    entropy: Some(entropy),
                    severity: "MEDIUM".to_owned(),
                });
            }
        }
    }
}

impl SecretRecognizer for EntropyRecognizer {
    fn name(&self) -> &'static str {
        "EntropyRecognizer"
    }

    fn base_score(&self) -> u8 {
        60 // Medium confidence - entropy alone is not definitive
    }

    fn scan_text(&self, content: &str, _file_path: &PathBuf) -> Vec<RawFinding> {
        let mut findings = Vec::new();

        for (line_idx, line) in content.lines().enumerate() {
            // Only scan comments for entropy
            if let Some((_, comment)) = line.split_once('#') {
                self.check_string(comment.trim(), line_idx + 1, &mut findings);
            }
        }

        findings
    }

    fn scan_text_fallback(&self, content: &str, _file_path: &PathBuf) -> Vec<RawFinding> {
        let mut findings = Vec::new();

        for (line_idx, line) in content.lines().enumerate() {
            for literal in Self::extract_string_literals(line) {
                self.check_string(literal, line_idx + 1, &mut findings);
            }

            if let Some((_, comment)) = line.split_once('#') {
                self.check_string(comment.trim(), line_idx + 1, &mut findings);
            }
        }

        findings
    }

    fn scan_ast(
        &self,
        stmts: &[Stmt],
        _file_path: &PathBuf,
        line_index: &LineIndex,
    ) -> Vec<RawFinding> {
        let mut findings = Vec::new();
        self.visit_stmts(stmts, line_index, &mut findings);
        findings
    }
}
