use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::SecretsConfig;

/// Represents a secret finding (e.g., a hardcoded API key).
#[derive(Debug, Clone, Serialize)]
pub struct SecretFinding {
    /// Description of the finding.
    pub message: String,
    /// Unique rule identifier (e.g., "CSP-S101").
    pub rule_id: String,
    /// File where the secret was found.
    pub file: PathBuf,
    /// Line number.
    pub line: usize,
    /// Severity level (e.g., "HIGH").
    pub severity: String,
    /// The matched value (redacted for security).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_value: Option<String>,
    /// Entropy score (if entropy-based detection).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entropy: Option<f64>,
}

/// Built-in secret pattern definition.
struct BuiltinPattern {
    name: &'static str,
    regex: Regex,
    rule_id: &'static str,
    severity: &'static str,
}

fn get_builtin_patterns() -> &'static Vec<BuiltinPattern> {
    use std::sync::OnceLock;
    static PATTERNS: OnceLock<Vec<BuiltinPattern>> = OnceLock::new();
    #[allow(clippy::unwrap_used)]
    PATTERNS.get_or_init(|| vec![
        // AWS Access Key ID
        BuiltinPattern {
            name: "AWS Access Key",
            regex: Regex::new(r#"(?i)aws_access_key_id\s*=\s*['"][A-Z0-9]{20}['"]"#).unwrap(),
            rule_id: "CSP-S101",
            severity: "HIGH",
        },
        // AWS Secret Access Key
        BuiltinPattern {
            name: "AWS Secret Key",
            regex: Regex::new(r#"(?i)aws_secret_access_key\s*=\s*['"][A-Za-z0-9/+=]{40}['"]"#).unwrap(),
            rule_id: "CSP-S102",
            severity: "CRITICAL",
        },
        // Generic API Key
        BuiltinPattern {
            name: "Generic API Key",
            regex: Regex::new(r#"(?i)(api_key|apikey|secret|token)\s*=\s*['"][A-Za-z0-9_\-]{20,}['"]"#).unwrap(),
            rule_id: "CSP-S103",
            severity: "HIGH",
        },
        // GitHub Token
        BuiltinPattern {
            name: "GitHub Token",
            regex: Regex::new(r"ghp_[a-zA-Z0-9]{36}").unwrap(),
            rule_id: "CSP-S104",
            severity: "CRITICAL",
        },
        // GitHub OAuth Token
        BuiltinPattern {
            name: "GitHub OAuth Token",
            regex: Regex::new(r"gho_[a-zA-Z0-9]{36}").unwrap(),
            rule_id: "CSP-S105",
            severity: "CRITICAL",
        },
        // GitHub App Token
        BuiltinPattern {
            name: "GitHub App Token",
            regex: Regex::new(r"(ghu|ghs)_[a-zA-Z0-9]{36}").unwrap(),
            rule_id: "CSP-S106",
            severity: "CRITICAL",
        },
        // GitLab Personal Access Token
        BuiltinPattern {
            name: "GitLab PAT",
            regex: Regex::new(r"glpat-[a-zA-Z0-9\-]{20}").unwrap(),
            rule_id: "CSP-S107",
            severity: "CRITICAL",
        },
        // Slack Bot Token
        BuiltinPattern {
            name: "Slack Bot Token",
            regex: Regex::new(r"xoxb-[a-zA-Z0-9-]{10,}").unwrap(),
            rule_id: "CSP-S108",
            severity: "HIGH",
        },
        // Slack User Token
        BuiltinPattern {
            name: "Slack User Token",
            regex: Regex::new(r"xoxp-[a-zA-Z0-9-]{10,}").unwrap(),
            rule_id: "CSP-S109",
            severity: "HIGH",
        },
        // Stripe Live Key
        BuiltinPattern {
            name: "Stripe Live Key",
            regex: Regex::new(r"sk_live_[a-zA-Z0-9]{24}").unwrap(),
            rule_id: "CSP-S110",
            severity: "CRITICAL",
        },
        // Stripe Test Key (lower severity)
        BuiltinPattern {
            name: "Stripe Test Key",
            regex: Regex::new(r"sk_test_[a-zA-Z0-9]{24}").unwrap(),
            rule_id: "CSP-S111",
            severity: "MEDIUM",
        },
        // Private Key
        BuiltinPattern {
            name: "Private Key",
            regex: Regex::new(r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----").unwrap(),
            rule_id: "CSP-S112",
            severity: "CRITICAL",
        },
        // Google API Key
        BuiltinPattern {
            name: "Google API Key",
            regex: Regex::new(r"AIza[0-9A-Za-z\-_]{35}").unwrap(),
            rule_id: "CSP-S113",
            severity: "HIGH",
        },
        // Heroku API Key
        BuiltinPattern {
            name: "Heroku API Key",
            regex: Regex::new(r#"(?i)heroku[_-]?api[_-]?key\s*[=:]\s*['"][0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}['"]"#).unwrap(),
            rule_id: "CSP-S114",
            severity: "HIGH",
        },
        // SendGrid API Key
        BuiltinPattern {
            name: "SendGrid API Key",
            regex: Regex::new(r"SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43}").unwrap(),
            rule_id: "CSP-S115",
            severity: "HIGH",
        },
        // Twilio API Key
        BuiltinPattern {
            name: "Twilio API Key",
            regex: Regex::new(r"SK[a-f0-9]{32}").unwrap(),
            rule_id: "CSP-S116",
            severity: "HIGH",
        },
        // NPM Token
        BuiltinPattern {
            name: "NPM Token",
            regex: Regex::new(r"npm_[a-zA-Z0-9]{36}").unwrap(),
            rule_id: "CSP-S117",
            severity: "HIGH",
        },
        // PyPI Token
        BuiltinPattern {
            name: "PyPI Token",
            regex: Regex::new(r"pypi-[a-zA-Z0-9_-]{50,}").unwrap(),
            rule_id: "CSP-S118",
            severity: "HIGH",
        },
        // Discord Token
        BuiltinPattern {
            name: "Discord Token",
            regex: Regex::new(r"[MN][A-Za-z\d]{23,}\.[\w-]{6}\.[\w-]{27}").unwrap(),
            rule_id: "CSP-S119",
            severity: "HIGH",
        },
        // Database Connection String
        BuiltinPattern {
            name: "Database Connection String",
            regex: Regex::new(r"(?i)(mysql|postgres|mongodb|redis)://[^:]+:[^@]+@[^\s]+").unwrap(),
            rule_id: "CSP-S120",
            severity: "CRITICAL",
        },
    ])
}

// ============================================================================
// Shannon Entropy Calculation
// ============================================================================

/// Calculates Shannon entropy of a string.
/// Returns a value between 0.0 (no randomness) and 8.0 (maximum randomness for byte data).
///
/// Typical values:
/// - English text: ~3.5-4.5
/// - Random alphanumeric: ~5.5-6.0
/// - API keys/secrets: ~4.5-6.0
/// - Variable names: ~2.5-4.0
pub fn calculate_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }

    let mut char_counts: HashMap<char, usize> = HashMap::new();
    let len = s.len() as f64;

    for c in s.chars() {
        *char_counts.entry(c).or_insert(0) += 1;
    }

    let entropy: f64 = char_counts
        .values()
        .map(|&count| {
            let p = count as f64 / len;
            -p * p.log2()
        })
        .sum();

    entropy
}

/// Checks if a string has high entropy (likely random/secret).
pub fn is_high_entropy(s: &str, threshold: f64, min_length: usize) -> bool {
    if s.len() < min_length {
        return false;
    }
    calculate_entropy(s) >= threshold
}

/// Extracts quoted strings from a line for entropy analysis.
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

/// Redacts a secret value for safe display (shows first 4 and last 4 chars).
fn redact_value(s: &str) -> String {
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

// ============================================================================
// Main Scanning Functions
// ============================================================================

/// Scans the content of a file for secrets using regex patterns and entropy analysis.
///
/// If `docstring_lines` is provided and `config.skip_docstrings` is true,
/// lines in that set will be skipped during entropy-based detection.
pub fn scan_secrets(
    content: &str,
    file_path: &PathBuf,
    config: &SecretsConfig,
    docstring_lines: Option<&rustc_hash::FxHashSet<usize>>,
) -> Vec<SecretFinding> {
    let mut findings = Vec::new();

    // Compile custom patterns
    let custom_patterns: Vec<(String, Regex, String, String)> = config
        .patterns
        .iter()
        .filter_map(|p| {
            Regex::new(&p.regex).ok().map(|r| {
                (
                    p.name.clone(),
                    r,
                    p.rule_id
                        .clone()
                        .unwrap_or_else(|| format!("CSP-CUSTOM-{}", p.name.replace(' ', "-"))),
                    p.severity.clone(),
                )
            })
        })
        .collect();

    for (line_idx, line) in content.lines().enumerate() {
        // Skip full-line comments if scan_comments is disabled
        // By default, comments ARE scanned to catch accidentally committed secrets
        if !config.scan_comments && line.trim().starts_with('#') {
            continue;
        }

        // Check for pragma: no cytoscnpy
        if line.contains("pragma: no cytoscnpy") {
            continue;
        }

        // 1. Check built-in patterns
        for pattern in get_builtin_patterns() {
            if pattern.regex.is_match(line) {
                findings.push(SecretFinding {
                    message: format!("Found potential {}", pattern.name),
                    rule_id: pattern.rule_id.to_owned(),
                    file: file_path.clone(),
                    line: line_idx + 1,
                    severity: pattern.severity.to_owned(),
                    matched_value: None,
                    entropy: None,
                });
            }
        }

        // 2. Check custom patterns from config
        for (name, regex, rule_id, severity) in &custom_patterns {
            if regex.is_match(line) {
                findings.push(SecretFinding {
                    message: format!("Found potential {name} (custom pattern)"),
                    rule_id: rule_id.clone(),
                    file: file_path.clone(),
                    line: line_idx + 1,
                    severity: severity.clone(),
                    matched_value: None,
                    entropy: None,
                });
            }
        }

        // 3. Entropy-based detection for high-entropy strings
        if config.entropy_enabled {
            // Skip this line if it's a docstring and skip_docstrings is enabled
            let is_docstring_line = config.skip_docstrings
                && docstring_lines
                    .map(|lines| lines.contains(&(line_idx + 1)))
                    .unwrap_or(false);

            if !is_docstring_line {
                for literal in extract_string_literals(line) {
                    if is_high_entropy(literal, config.entropy_threshold, config.min_length) {
                        let entropy = calculate_entropy(literal);
                        // Additional filter: skip if it looks like a path or URL
                        if !looks_like_path_or_url(literal) {
                            findings.push(SecretFinding {
                                message: format!(
                                    "High-entropy string detected (entropy: {entropy:.2})"
                                ),
                                rule_id: "CSP-S200".to_owned(),
                                file: file_path.clone(),
                                line: line_idx + 1,
                                severity: "MEDIUM".to_owned(),
                                matched_value: Some(redact_value(literal)),
                                entropy: Some(entropy),
                            });
                        }
                    }
                }
            }
        }
    }

    findings
}

/// Checks if a string looks like a file path or URL (to reduce false positives).
fn looks_like_path_or_url(s: &str) -> bool {
    // URL patterns
    if s.starts_with("http://") || s.starts_with("https://") || s.starts_with("ftp://") {
        return true;
    }
    // File path patterns
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

/// Backward-compatible scan function (uses default config, no docstring filtering).
pub fn scan_secrets_compat(content: &str, file_path: &PathBuf) -> Vec<SecretFinding> {
    scan_secrets(content, file_path, &SecretsConfig::default(), None)
}

// ============================================================================
// Tests
// ============================================================================
