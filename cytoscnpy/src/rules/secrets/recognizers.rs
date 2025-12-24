//! Pluggable secret recognizers.
//!
//! This module defines the `SecretRecognizer` trait and provides
//! implementations for different detection strategies.

use crate::config::SecretsConfig;
use crate::utils::LineIndex;
use regex::Regex;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;
use std::collections::HashMap;
use std::path::PathBuf;

use super::patterns::get_builtin_patterns;

// ============================================================================
// Raw Finding (before scoring)
// ============================================================================

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

// ============================================================================
// Recognizer Trait
// ============================================================================

/// Trait for pluggable secret recognizers.
///
/// Recognizers can scan text content and/or AST nodes to detect secrets.
pub trait SecretRecognizer: Send + Sync {
    /// Name of the recognizer for logging/debugging.
    fn name(&self) -> &str;

    /// Base confidence score (0-100) for findings from this recognizer.
    fn base_score(&self) -> u8;

    /// Scan text content for secrets. Returns raw findings before scoring.
    fn scan_text(&self, content: &str, file_path: &PathBuf) -> Vec<RawFinding>;

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

// ============================================================================
// Regex Recognizer
// ============================================================================

/// Regex-based pattern matching recognizer.
///
/// Uses built-in patterns to detect known secret formats.
pub struct RegexRecognizer;

impl SecretRecognizer for RegexRecognizer {
    fn name(&self) -> &str {
        "RegexRecognizer"
    }

    fn base_score(&self) -> u8 {
        85 // High confidence for pattern matches
    }

    fn scan_text(&self, content: &str, _file_path: &PathBuf) -> Vec<RawFinding> {
        let mut findings = Vec::new();
        let patterns = get_builtin_patterns();

        for (line_idx, line) in content.lines().enumerate() {
            for pattern in patterns {
                if pattern.regex.is_match(line) {
                    findings.push(RawFinding {
                        message: format!("Found potential {}", pattern.name),
                        rule_id: pattern.rule_id.to_owned(),
                        line: line_idx + 1,
                        base_score: pattern.base_score,
                        matched_value: None,
                        entropy: None,
                        severity: pattern.severity.to_owned(),
                    });
                }
            }
        }

        findings
    }
}

// ============================================================================
// Entropy Recognizer
// ============================================================================

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
    fn extract_string_literals(line: &str) -> Vec<&str> {
        let mut strings = Vec::new();
        let mut in_string = false;
        let mut quote_char = ' ';
        let mut start = 0;

        for (i, c) in line.char_indices() {
            if !in_string && (c == '"' || c == '\'') {
                in_string = true;
                quote_char = c;
                start = i + 1;
            } else if in_string && c == quote_char {
                if i > start {
                    strings.push(&line[start..i]);
                }
                in_string = false;
            }
        }

        strings
    }

    /// Check if a string looks like a path or URL.
    fn looks_like_path_or_url(s: &str) -> bool {
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
}

impl SecretRecognizer for EntropyRecognizer {
    fn name(&self) -> &str {
        "EntropyRecognizer"
    }

    fn base_score(&self) -> u8 {
        60 // Medium confidence - entropy alone is not definitive
    }

    fn scan_text(&self, content: &str, _file_path: &PathBuf) -> Vec<RawFinding> {
        let mut findings = Vec::new();

        for (line_idx, line) in content.lines().enumerate() {
            for literal in Self::extract_string_literals(line) {
                if literal.len() >= self.min_length {
                    let entropy = Self::calculate_entropy(literal);
                    if entropy >= self.threshold && !Self::looks_like_path_or_url(literal) {
                        findings.push(RawFinding {
                            message: format!(
                                "High-entropy string detected (entropy: {entropy:.2})"
                            ),
                            rule_id: "CSP-S200".to_owned(),
                            line: line_idx + 1,
                            base_score: self.base_score(),
                            matched_value: Some(Self::redact_value(literal)),
                            entropy: Some(entropy),
                            severity: "MEDIUM".to_owned(),
                        });
                    }
                }
            }
        }

        findings
    }
}

// ============================================================================
// AST Recognizer
// ============================================================================

/// Suspicious variable name patterns.
const SUSPICIOUS_NAMES: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "secret",
    "key",
    "token",
    "auth",
    "credential",
    "api_key",
    "apikey",
    "private_key",
    "access_token",
    "secret_key",
    "auth_token",
    "bearer",
    "client_secret",
    "app_secret",
    "encryption_key",
    "signing_key",
    "master_key",
];

/// AST-based suspicious variable name detection recognizer.
pub struct AstRecognizer {
    /// Additional suspicious names from config.
    custom_names: Vec<String>,
}

impl Default for AstRecognizer {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl AstRecognizer {
    /// Creates a new AST recognizer with custom suspicious names.
    #[must_use]
    pub fn new(custom_names: Vec<String>) -> Self {
        Self { custom_names }
    }

    /// Check if a name matches suspicious patterns.
    fn matches_suspicious_name(&self, name: &str) -> bool {
        let lower = name.to_lowercase();

        // Check built-in patterns
        if SUSPICIOUS_NAMES.iter().any(|s| lower.contains(s)) {
            return true;
        }

        // Check custom patterns
        self.custom_names
            .iter()
            .any(|s| lower.contains(&s.to_lowercase()))
    }

    /// Extract string value from an expression if it's a literal string.
    fn extract_string_value(expr: &Expr) -> Option<String> {
        match expr {
            Expr::StringLiteral(s) => Some(s.value.to_string()),
            _ => None,
        }
    }

    /// Check if the value is from an environment variable access.
    fn is_env_var_access(expr: &Expr) -> bool {
        match expr {
            Expr::Call(call) => {
                // Check for os.environ.get(...) or os.getenv(...)
                match &*call.func {
                    Expr::Attribute(attr) => {
                        let attr_name = attr.attr.as_str();
                        if attr_name == "get" {
                            // Check if it's environ.get
                            if let Expr::Attribute(inner) = &*attr.value {
                                return inner.attr.as_str() == "environ";
                            }
                        }
                        if attr_name == "getenv" {
                            // Check if it's os.getenv
                            if let Expr::Name(name) = &*attr.value {
                                return name.id.as_str() == "os";
                            }
                        }
                        false
                    }
                    Expr::Name(name) => {
                        // Direct getenv call (from os import getenv)
                        name.id.as_str() == "getenv"
                    }
                    _ => false,
                }
            }
            Expr::Subscript(sub) => {
                // Check for os.environ[...]
                if let Expr::Attribute(attr) = &*sub.value {
                    return attr.attr.as_str() == "environ";
                }
                false
            }
            _ => false,
        }
    }

    /// Redact a secret value.
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

    /// Check if value looks like a placeholder.
    fn is_placeholder(value: &str) -> bool {
        let lower = value.to_lowercase();
        lower.starts_with("xxx")
            || lower.starts_with("your_")
            || lower.starts_with("changeme")
            || lower.starts_with("replace_")
            || lower.starts_with("example")
            || lower.starts_with("<")
            || lower.contains("${")
            || lower.contains("{{")
            || lower == "none"
            || lower == "null"
            || lower.is_empty()
    }

    /// Process an assignment statement.
    fn process_assign(
        &self,
        targets: &[Expr],
        value: &Expr,
        line: usize,
        findings: &mut Vec<RawFinding>,
    ) {
        // Skip if value is from environment variable
        if Self::is_env_var_access(value) {
            return;
        }

        // Skip if value is not a string literal
        let string_value = match Self::extract_string_value(value) {
            Some(v) => v,
            None => return,
        };

        // Skip placeholders
        if Self::is_placeholder(&string_value) {
            return;
        }

        for target in targets {
            let name = match target {
                // Simple assignment: x = "value"
                Expr::Name(name) => name.id.to_string(),
                // Attribute assignment: self.x = "value"
                Expr::Attribute(attr) => attr.attr.to_string(),
                // Subscript assignment: config["x"] = "value"
                Expr::Subscript(sub) => {
                    if let Expr::StringLiteral(key) = &*sub.slice {
                        key.value.to_string()
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };

            if self.matches_suspicious_name(&name) {
                findings.push(RawFinding {
                    message: format!("Suspicious assignment to '{name}'"),
                    rule_id: "CSP-S300".to_owned(),
                    line,
                    base_score: 70,
                    matched_value: Some(Self::redact_value(&string_value)),
                    entropy: None,
                    severity: "MEDIUM".to_owned(),
                });
            }
        }
    }

    /// Recursively visit statements to find suspicious assignments.
    fn visit_stmts(&self, stmts: &[Stmt], line_index: &LineIndex, findings: &mut Vec<RawFinding>) {
        for stmt in stmts {
            self.visit_stmt(stmt, line_index, findings);
        }
    }

    /// Visit a single statement.
    fn visit_stmt(&self, stmt: &Stmt, line_index: &LineIndex, findings: &mut Vec<RawFinding>) {
        match stmt {
            Stmt::Assign(node) => {
                let line = line_index.line_index(node.start());
                self.process_assign(&node.targets, &node.value, line, findings);
            }
            Stmt::AnnAssign(node) => {
                if let Some(value) = &node.value {
                    let line = line_index.line_index(node.start());
                    self.process_assign(&[(*node.target).clone()], value, line, findings);
                }
            }
            // Recurse into compound statements
            Stmt::FunctionDef(node) => {
                self.visit_stmts(&node.body, line_index, findings);
            }
            Stmt::ClassDef(node) => {
                self.visit_stmts(&node.body, line_index, findings);
            }
            Stmt::If(node) => {
                self.visit_stmts(&node.body, line_index, findings);
                for clause in &node.elif_else_clauses {
                    self.visit_stmts(&clause.body, line_index, findings);
                }
            }
            Stmt::For(node) => {
                self.visit_stmts(&node.body, line_index, findings);
                self.visit_stmts(&node.orelse, line_index, findings);
            }
            Stmt::While(node) => {
                self.visit_stmts(&node.body, line_index, findings);
                self.visit_stmts(&node.orelse, line_index, findings);
            }
            Stmt::With(node) => {
                self.visit_stmts(&node.body, line_index, findings);
            }
            Stmt::Try(node) => {
                self.visit_stmts(&node.body, line_index, findings);
                for ast::ExceptHandler::ExceptHandler(h) in &node.handlers {
                    self.visit_stmts(&h.body, line_index, findings);
                }
                self.visit_stmts(&node.orelse, line_index, findings);
                self.visit_stmts(&node.finalbody, line_index, findings);
            }
            Stmt::Match(node) => {
                for case in &node.cases {
                    self.visit_stmts(&case.body, line_index, findings);
                }
            }
            _ => {}
        }
    }
}

impl SecretRecognizer for AstRecognizer {
    fn name(&self) -> &str {
        "AstRecognizer"
    }

    fn base_score(&self) -> u8 {
        70 // Medium-high confidence
    }

    fn scan_text(&self, _content: &str, _file_path: &PathBuf) -> Vec<RawFinding> {
        // AST recognizer doesn't use text scanning
        Vec::new()
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

// ============================================================================
// Custom Recognizer
// ============================================================================

/// User-defined custom pattern recognizer.
pub struct CustomRecognizer {
    /// List of (name, regex, rule_id, severity, score) patterns.
    patterns: Vec<(String, Regex, String, String, u8)>,
}

impl CustomRecognizer {
    /// Creates a new custom recognizer from config.
    #[must_use]
    pub fn new(config: &SecretsConfig) -> Self {
        let mut patterns = Vec::new();

        for p in &config.patterns {
            if let Ok(regex) = Regex::new(&p.regex) {
                let rule_id = p
                    .rule_id
                    .clone()
                    .unwrap_or_else(|| format!("CSP-CUSTOM-{}", p.name.replace(' ', "-")));
                patterns.push((p.name.clone(), regex, rule_id, p.severity.clone(), 75));
            }
        }

        Self { patterns }
    }
}

impl SecretRecognizer for CustomRecognizer {
    fn name(&self) -> &str {
        "CustomRecognizer"
    }

    fn base_score(&self) -> u8 {
        75 // Default score for custom patterns
    }

    fn scan_text(&self, content: &str, _file_path: &PathBuf) -> Vec<RawFinding> {
        let mut findings = Vec::new();

        for (line_idx, line) in content.lines().enumerate() {
            for (name, regex, rule_id, severity, score) in &self.patterns {
                if regex.is_match(line) {
                    findings.push(RawFinding {
                        message: format!("Found potential {name} (custom pattern)"),
                        rule_id: rule_id.clone(),
                        line: line_idx + 1,
                        base_score: *score,
                        matched_value: None,
                        entropy: None,
                        severity: severity.clone(),
                    });
                }
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_recognizer_github_token() {
        let recognizer = RegexRecognizer;
        let content = "token = 'ghp_abcdefghijklmnopqrstuvwxyz1234567890'";
        let findings = recognizer.scan_text(content, &PathBuf::from("test.py"));

        // May match multiple patterns (GitHub Token + Generic API Key)
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "CSP-S104"));
        assert!(findings.iter().any(|f| f.message.contains("GitHub Token")));
    }

    #[test]
    fn test_entropy_recognizer() {
        let recognizer = EntropyRecognizer::default();
        // High entropy string
        let content = "api_key = 'aB3xY7mN9pQ2rS5tU8vW0zK4cF6gH1jL'";
        let findings = recognizer.scan_text(content, &PathBuf::from("test.py"));

        assert!(!findings.is_empty());
        assert_eq!(findings[0].rule_id, "CSP-S200");
    }

    #[test]
    fn test_ast_recognizer_suspicious_name() {
        let recognizer = AstRecognizer::default();
        let code = r#"password = "secret123""#;

        let parsed = ruff_python_parser::parse_module(code).expect("Failed to parse");
        let line_index = LineIndex::new(code);

        let findings = recognizer.scan_ast(
            &parsed.into_syntax().body,
            &PathBuf::from("test.py"),
            &line_index,
        );

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "CSP-S300");
        assert!(findings[0].message.contains("password"));
    }

    #[test]
    fn test_ast_recognizer_skips_env_var() {
        let recognizer = AstRecognizer::default();
        let code = r#"password = os.environ.get("PASSWORD")"#;

        let parsed = ruff_python_parser::parse_module(code).expect("Failed to parse");
        let line_index = LineIndex::new(code);

        let findings = recognizer.scan_ast(
            &parsed.into_syntax().body,
            &PathBuf::from("test.py"),
            &line_index,
        );

        assert!(findings.is_empty());
    }

    #[test]
    fn test_ast_recognizer_dict_subscript() {
        let recognizer = AstRecognizer::default();
        let code = r#"config["api_key"] = "my_secret_token""#;

        let parsed = ruff_python_parser::parse_module(code).expect("Failed to parse");
        let line_index = LineIndex::new(code);

        let findings = recognizer.scan_ast(
            &parsed.into_syntax().body,
            &PathBuf::from("test.py"),
            &line_index,
        );

        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("api_key"));
    }

    #[test]
    fn test_ast_recognizer_attribute() {
        let recognizer = AstRecognizer::default();
        let code = r#"self.secret_key = "my_secret""#;

        let parsed = ruff_python_parser::parse_module(code).expect("Failed to parse");
        let line_index = LineIndex::new(code);

        let findings = recognizer.scan_ast(
            &parsed.into_syntax().body,
            &PathBuf::from("test.py"),
            &line_index,
        );

        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("secret_key"));
    }
}
