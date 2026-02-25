use crate::config::SecretsConfig;
use crate::utils::LineIndex;
use rustc_hash::FxHashSet;
use std::path::PathBuf;

use super::finding::SecretFinding;
use super::scanner::SecretScanner;

/// Validates custom regex patterns in the secrets configuration.
#[must_use]
pub fn validate_secrets_config(
    config: &SecretsConfig,
    config_file_path: &PathBuf,
) -> Vec<SecretFinding> {
    let mut findings = Vec::new();
    for pattern in &config.patterns {
        if let Err(err) = regex::Regex::new(&pattern.regex) {
            findings.push(SecretFinding {
                message: format!(
                    "Invalid regex for custom secret pattern '{}': {}",
                    pattern.name, err
                ),
                rule_id: crate::constants::RULE_ID_CONFIG_ERROR.to_owned(),
                category: "Secrets".to_owned(),
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

/// Scans file content for secrets using regex/AST/entropy analysis.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn scan_secrets(
    content: &str,
    file_path: &PathBuf,
    config: &SecretsConfig,
    docstring_lines: Option<&FxHashSet<usize>>,
    is_test_file: bool,
) -> Vec<SecretFinding> {
    let scanner = SecretScanner::new(config);
    let line_index = LineIndex::new(content);

    let stmts = ruff_python_parser::parse_module(content)
        .ok()
        .map(|parsed| parsed.into_syntax().body);

    scanner.scan(
        content,
        stmts.as_deref(),
        file_path,
        &line_index,
        docstring_lines,
        is_test_file,
    )
}

/// Backward-compatible scan function (default config, no docstring filtering).
#[must_use]
pub fn scan_secrets_compat(content: &str, file_path: &PathBuf) -> Vec<SecretFinding> {
    let is_test_file = crate::utils::is_test_path(&file_path.to_string_lossy());
    scan_secrets(
        content,
        file_path,
        &SecretsConfig::default(),
        None,
        is_test_file,
    )
}
