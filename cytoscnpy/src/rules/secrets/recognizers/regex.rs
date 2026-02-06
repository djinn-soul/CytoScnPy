use super::super::patterns::get_builtin_patterns;
use super::types::{RawFinding, SecretRecognizer};
use std::path::PathBuf;

/// Regex-based pattern matching recognizer.
///
/// Uses built-in patterns to detect known secret formats.
pub struct RegexRecognizer;

impl SecretRecognizer for RegexRecognizer {
    fn name(&self) -> &'static str {
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
