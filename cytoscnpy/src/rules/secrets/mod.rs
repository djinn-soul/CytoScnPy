//! Modular Secret Recognition Engine.
//!
//! This module provides a pluggable, trait-based architecture for detecting
//! secrets and sensitive data in Python code. It supports:
//!
//! - **Regex patterns**: Detect known secret formats (API keys, tokens, etc.)
//! - **AST analysis**: Detect suspicious variable assignments
//! - **Entropy analysis**: Detect high-entropy strings
//! - **Custom patterns**: User-defined detection rules
//!
//! All findings go through a context-based scoring system that adjusts
//! confidence based on signals like file type, proximity to keywords, etc.

mod patterns;
mod recognizers;
mod scoring;

pub use patterns::{get_builtin_patterns, BuiltinPattern};
pub use recognizers::{
    AstRecognizer, CustomRecognizer, EntropyRecognizer, RawFinding, RegexRecognizer,
    SecretRecognizer,
};
pub use scoring::{ContextScorer, ScoringAdjustments, ScoringContext};

use crate::config::SecretsConfig;
use crate::constants::RULE_ID_CONFIG_ERROR;
use crate::utils::LineIndex;
use ruff_python_ast::Stmt;
use rustc_hash::FxHashSet;
use serde::Serialize;
use std::path::PathBuf;

// ============================================================================
// Secret Finding
// ============================================================================

/// Represents a secret finding with confidence scoring.
#[derive(Debug, Clone, Serialize)]
pub struct SecretFinding {
    /// Description of the finding.
    pub message: String,
    /// Unique rule identifier (e.g., "CSP-S101").
    pub rule_id: String,
    /// File where the secret was found.
    pub file: PathBuf,
    /// Line number (1-indexed).
    pub line: usize,
    /// Severity level (e.g., "HIGH", "CRITICAL").
    pub severity: String,
    /// The matched value (redacted for security).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_value: Option<String>,
    /// Entropy score (if entropy-based detection).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entropy: Option<f64>,
    /// Confidence score (0-100). Higher = more confident it's a real secret.
    pub confidence: u8,
}

// ============================================================================
// Secret Scanner
// ============================================================================

/// Main secret scanner that orchestrates all recognizers.
///
/// The scanner:
/// 1. Runs all enabled recognizers to collect raw findings
/// 2. Applies context-based scoring to each finding
/// 3. Filters findings below the minimum score threshold
/// 4. Deduplicates findings on the same line
pub struct SecretScanner {
    /// Enabled recognizers.
    recognizers: Vec<Box<dyn SecretRecognizer>>,
    /// Context scorer for adjusting confidence.
    scorer: ContextScorer,
    /// Minimum score to report (0-100).
    min_score: u8,
    /// Whether to scan comments.
    scan_comments: bool,
}

impl SecretScanner {
    /// Creates a new secret scanner from configuration.
    ///
    /// By default, all recognizers are enabled:
    /// - `RegexRecognizer` (built-in patterns)
    /// - `AstRecognizer` (suspicious variable names) - enabled by default
    /// - `EntropyRecognizer` (high-entropy strings)
    /// - `CustomRecognizer` (user-defined patterns)
    #[must_use]
    pub fn new(config: &SecretsConfig) -> Self {
        let mut recognizers: Vec<Box<dyn SecretRecognizer>> = Vec::new();

        // Always add regex recognizer
        recognizers.push(Box::new(RegexRecognizer));

        // Add AST recognizer (enabled by default - no config needed)
        recognizers.push(Box::new(AstRecognizer::new(
            config.suspicious_names.clone(),
        )));

        // Add entropy recognizer if enabled
        if config.entropy_enabled {
            recognizers.push(Box::new(EntropyRecognizer::new(
                config.entropy_threshold,
                config.min_length,
            )));
        }

        // Add custom recognizer if patterns are defined
        if !config.patterns.is_empty() {
            recognizers.push(Box::new(CustomRecognizer::new(config)));
        }

        Self {
            recognizers,
            scorer: ContextScorer::new(),
            min_score: config.min_score,
            scan_comments: config.scan_comments,
        }
    }

    /// Scan content using all recognizers and apply scoring.
    ///
    /// # Arguments
    /// * `content` - The source code content
    /// * `stmts` - Optional parsed AST statements (for AST-based detection)
    /// * `file_path` - Path to the file being scanned
    /// * `line_index` - Line index for offset-to-line conversion
    /// * `docstring_lines` - Optional set of lines that are docstrings
    #[must_use]
    pub fn scan(
        &self,
        content: &str,
        stmts: Option<&[Stmt]>,
        file_path: &PathBuf,
        line_index: &LineIndex,
        docstring_lines: Option<&FxHashSet<usize>>,
    ) -> Vec<SecretFinding> {
        let mut all_findings = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Run all recognizers
        for recognizer in &self.recognizers {
            // Text-based scanning
            let text_findings = recognizer.scan_text(content, file_path);
            all_findings.extend(text_findings);

            // AST-based scanning (if AST is available)
            if let Some(stmts) = stmts {
                let ast_findings = recognizer.scan_ast(stmts, file_path, line_index);
                all_findings.extend(ast_findings);
            }
        }

        // Apply scoring and filtering
        let mut scored_findings = Vec::new();
        let mut seen_lines: FxHashSet<usize> = FxHashSet::default();

        for finding in all_findings {
            let line_idx = finding.line.saturating_sub(1);
            let line_content = lines.get(line_idx).unwrap_or(&"");

            // Skip if not scanning comments and line is a comment
            let is_comment = line_content.trim().starts_with('#');
            if !self.scan_comments && is_comment {
                continue;
            }

            // Skip suppressed lines (pragma or noqa comments)
            let patterns = crate::constants::SUPPRESSION_PATTERNS();
            if patterns.iter().any(|p| line_content.contains(p)) {
                continue;
            }

            // Check if in docstring
            let is_docstring = docstring_lines.is_some_and(|lines| lines.contains(&finding.line));

            // Create scoring context
            let ctx = ScoringContext {
                line_content,
                file_path,
                is_comment,
                is_docstring,
            };

            // Score the finding
            let confidence = self.scorer.score(finding.base_score, &ctx);

            // Filter by minimum score
            if confidence < self.min_score {
                continue;
            }

            // Deduplicate by line (keep highest confidence)
            if seen_lines.contains(&finding.line) {
                // Check if we should replace existing finding
                if let Some(existing) = scored_findings
                    .iter_mut()
                    .find(|f: &&mut SecretFinding| f.line == finding.line)
                {
                    if confidence > existing.confidence {
                        existing.message = finding.message;
                        existing.rule_id = finding.rule_id;
                        existing.severity = finding.severity;
                        existing.matched_value = finding.matched_value;
                        existing.entropy = finding.entropy;
                        existing.confidence = confidence;
                    }
                }
                continue;
            }

            seen_lines.insert(finding.line);

            scored_findings.push(SecretFinding {
                message: finding.message,
                rule_id: finding.rule_id,
                file: file_path.clone(),
                line: finding.line,
                severity: finding.severity,
                matched_value: finding.matched_value,
                entropy: finding.entropy,
                confidence,
            });
        }

        scored_findings
    }
}

// ============================================================================
// Backward-Compatible Functions
// ============================================================================

/// Validates custom regex patterns in the secrets configuration.
/// Returns a list of `SecretFinding` for any invalid patterns.
/// This should be called once at the start of analysis, not per-file.
#[must_use]
pub fn validate_secrets_config(
    config: &SecretsConfig,
    config_file_path: &PathBuf,
) -> Vec<SecretFinding> {
    let mut findings = Vec::new();
    for p in &config.patterns {
        if let Err(e) = regex::Regex::new(&p.regex) {
            findings.push(SecretFinding {
                message: format!(
                    "Invalid regex for custom secret pattern '{}': {}",
                    p.name, e
                ),
                rule_id: RULE_ID_CONFIG_ERROR.to_owned(),
                file: config_file_path.clone(),
                line: 1,
                severity: "CRITICAL".to_owned(),
                matched_value: None,
                entropy: None,
                confidence: 100,
            });
        }
    }
    findings
}

/// Scans the content of a file for secrets using regex patterns and entropy analysis.
///
/// This is the backward-compatible entry point that uses the new modular scanner.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn scan_secrets(
    content: &str,
    file_path: &PathBuf,
    config: &SecretsConfig,
    docstring_lines: Option<&FxHashSet<usize>>,
) -> Vec<SecretFinding> {
    let scanner = SecretScanner::new(config);
    let line_index = LineIndex::new(content);

    // Parse for AST-based detection
    let stmts = ruff_python_parser::parse_module(content)
        .ok()
        .map(|parsed| parsed.into_syntax().body);

    scanner.scan(
        content,
        stmts.as_deref(),
        file_path,
        &line_index,
        docstring_lines,
    )
}

/// Backward-compatible scan function (uses default config, no docstring filtering).
#[must_use]
pub fn scan_secrets_compat(content: &str, file_path: &PathBuf) -> Vec<SecretFinding> {
    scan_secrets(content, file_path, &SecretsConfig::default(), None)
}

// ============================================================================
// Utility Functions (re-exported for backward compatibility)
// ============================================================================

/// Calculates Shannon entropy of a string.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn calculate_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }

    let mut char_counts: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
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

/// Checks if a string has high entropy (likely random/secret).
#[must_use]
pub fn is_high_entropy(s: &str, threshold: f64, min_length: usize) -> bool {
    if s.len() < min_length {
        return false;
    }
    calculate_entropy(s) >= threshold
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> SecretsConfig {
        SecretsConfig::default()
    }

    #[test]
    fn test_scanner_detects_github_token() {
        let config = default_config();
        let content = "token = 'ghp_abcdefghijklmnopqrstuvwxyz1234567890'";
        let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None);

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "CSP-S104"));
    }

    #[test]
    fn test_scanner_detects_suspicious_variable() {
        let config = default_config();
        let content = r#"database_password = "super_secret_123""#;
        let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None);

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "CSP-S300"));
    }

    #[test]
    fn test_scanner_skips_env_var() {
        let config = default_config();
        let content = r#"password = os.environ.get("PASSWORD")"#;
        let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None);

        // Should not detect CSP-S300 for env var access
        assert!(
            !findings.iter().any(|f| f.rule_id == "CSP-S300"),
            "Should not detect env var access as suspicious"
        );
    }

    #[test]
    fn test_scanner_reduces_score_in_test_file() {
        let config = default_config();
        let content = r#"api_key = "test_secret_value_12345""#;

        // Normal file
        let normal_findings = scan_secrets(content, &PathBuf::from("src/main.py"), &config, None);

        // Test file
        let test_findings =
            scan_secrets(content, &PathBuf::from("tests/test_main.py"), &config, None);

        // Test file findings should have lower confidence
        if !normal_findings.is_empty() && !test_findings.is_empty() {
            let normal_conf = normal_findings[0].confidence;
            let test_conf = test_findings[0].confidence;
            assert!(
                test_conf < normal_conf,
                "Test file should have lower confidence: {test_conf} vs {normal_conf}"
            );
        }
    }

    #[test]
    fn test_scanner_deduplicates_findings() {
        let config = default_config();
        // This line matches both regex pattern AND suspicious variable name
        let content = r#"api_key = "ghp_abcdefghijklmnopqrstuvwxyz1234567890""#;
        let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None);

        // Should only have one finding per line (highest confidence)
        let line_1_findings: Vec<_> = findings.iter().filter(|f| f.line == 1).collect();
        assert_eq!(
            line_1_findings.len(),
            1,
            "Should deduplicate to one finding per line"
        );
    }

    #[test]
    fn test_entropy_calculation() {
        // Low entropy (repeated chars)
        assert!(calculate_entropy("aaaaaaaaaa") < 1.0);

        // Higher entropy (mixed chars)
        assert!(calculate_entropy("aB3xY7mN9p") > 3.0);

        // Empty string
        assert!((calculate_entropy("") - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_backward_compat_function() {
        let content = "token = 'ghp_abcdefghijklmnopqrstuvwxyz1234567890'";
        let findings = scan_secrets_compat(content, &PathBuf::from("test.py"));

        assert!(!findings.is_empty());
    }
    #[test]
    fn test_invalid_custom_regex_reporting() {
        use crate::config::{CustomSecretPattern, SecretsConfig};

        let secrets_config = SecretsConfig {
            patterns: vec![CustomSecretPattern {
                name: "Invalid Regex".to_owned(),
                regex: "[".to_owned(), // Invalid regex
                rule_id: None,
                severity: "CRITICAL".to_owned(),
            }],
            ..SecretsConfig::default()
        };

        let config_file = PathBuf::from(".cytoscnpy.toml");
        let findings = validate_secrets_config(&secrets_config, &config_file);

        // Should report a finding for invalid regex configuration
        assert!(
            !findings.is_empty(),
            "Should report a finding for invalid regex configuration"
        );
        assert_eq!(findings[0].rule_id, RULE_ID_CONFIG_ERROR);
        assert_eq!(findings[0].file, config_file);
    }
}
