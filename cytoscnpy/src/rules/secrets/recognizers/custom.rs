use super::types::{RawFinding, SecretRecognizer};
use crate::config::SecretsConfig;
use regex::Regex;
use std::path::PathBuf;

/// User-defined custom pattern recognizer.
pub struct CustomRecognizer {
    /// List of `(name, regex, rule_id, severity, score)` patterns.
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
    fn name(&self) -> &'static str {
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
