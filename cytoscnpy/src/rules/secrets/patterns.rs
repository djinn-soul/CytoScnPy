//! Built-in secret patterns for regex-based detection.
//!
//! This module contains all the pre-defined patterns for detecting
//! common secret formats (API keys, tokens, credentials, etc.).

use regex::Regex;
use std::sync::OnceLock;

/// Built-in secret pattern definition.
pub struct BuiltinPattern {
    /// Human-readable name of the secret type.
    pub name: &'static str,
    /// Compiled regex pattern.
    pub regex: Regex,
    /// Unique rule identifier (e.g., "CSP-S101").
    pub rule_id: &'static str,
    /// Severity level (e.g., "HIGH", "CRITICAL").
    pub severity: &'static str,
    /// Base confidence score (0-100) for this pattern.
    pub base_score: u8,
}

/// Returns the list of built-in secret patterns.
///
/// Patterns are compiled once and cached for the lifetime of the program.
///
/// # Panics
///
/// Panics if any of the built-in regex patterns fail to compile. This should
/// never happen in practice as all patterns are statically validated by tests.
fn try_init_patterns() -> Result<Vec<BuiltinPattern>, regex::Error> {
    let mut patterns = Vec::new();
    patterns.extend(get_cloud_patterns()?);
    patterns.extend(get_scm_patterns()?);
    patterns.extend(get_service_patterns()?);
    patterns.extend(get_generic_patterns()?);
    Ok(patterns)
}

fn get_cloud_patterns() -> Result<Vec<BuiltinPattern>, regex::Error> {
    Ok(vec![
        // AWS Access Key ID
        BuiltinPattern {
            name: "AWS Access Key",
            regex: Regex::new(r#"(?i)aws_access_key_id\s*=\s*['"][A-Z0-9]{20}['"]"#)?,
            rule_id: "CSP-S101",
            severity: "HIGH",
            base_score: 90,
        },
        // AWS Secret Access Key
        BuiltinPattern {
            name: "AWS Secret Key",
            regex: Regex::new(r#"(?i)aws_secret_access_key\s*=\s*['"][A-Za-z0-9/+=]{40}['"]"#)?,
            rule_id: "CSP-S102",
            severity: "CRITICAL",
            base_score: 95,
        },
        // Google API Key
        BuiltinPattern {
            name: "Google API Key",
            regex: Regex::new(r"AIza[0-9A-Za-z\-_]{35}")?,
            rule_id: "CSP-S113",
            severity: "HIGH",
            base_score: 90,
        },
        // Heroku API Key
        BuiltinPattern {
            name: "Heroku API Key",
            regex: Regex::new(
                r#"(?i)heroku[_-]?api[_-]?key\s*[=:]\s*['"][0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}['"]"#,
            )?,
            rule_id: "CSP-S114",
            severity: "HIGH",
            base_score: 90,
        },
    ])
}

fn get_scm_patterns() -> Result<Vec<BuiltinPattern>, regex::Error> {
    Ok(vec![
        // GitHub Token
        BuiltinPattern {
            name: "GitHub Token",
            regex: Regex::new(r"ghp_[a-zA-Z0-9]{36}")?,
            rule_id: "CSP-S104",
            severity: "CRITICAL",
            base_score: 95,
        },
        // GitHub OAuth Token
        BuiltinPattern {
            name: "GitHub OAuth Token",
            regex: Regex::new(r"gho_[a-zA-Z0-9]{36}")?,
            rule_id: "CSP-S105",
            severity: "CRITICAL",
            base_score: 95,
        },
        // GitHub App Token
        BuiltinPattern {
            name: "GitHub App Token",
            regex: Regex::new(r"(ghu|ghs)_[a-zA-Z0-9]{36}")?,
            rule_id: "CSP-S106",
            severity: "CRITICAL",
            base_score: 95,
        },
        // GitLab Personal Access Token
        BuiltinPattern {
            name: "GitLab PAT",
            regex: Regex::new(r"glpat-[a-zA-Z0-9\-]{20}")?,
            rule_id: "CSP-S107",
            severity: "CRITICAL",
            base_score: 95,
        },
    ])
}

fn get_service_patterns() -> Result<Vec<BuiltinPattern>, regex::Error> {
    Ok(vec![
        // Slack Bot Token
        BuiltinPattern {
            name: "Slack Bot Token",
            regex: Regex::new(r"xoxb-[a-zA-Z0-9-]{10,}")?,
            rule_id: "CSP-S108",
            severity: "HIGH",
            base_score: 90,
        },
        // Slack User Token
        BuiltinPattern {
            name: "Slack User Token",
            regex: Regex::new(r"xoxp-[a-zA-Z0-9-]{10,}")?,
            rule_id: "CSP-S109",
            severity: "HIGH",
            base_score: 90,
        },
        // Stripe Live Key
        BuiltinPattern {
            name: "Stripe Live Key",
            regex: Regex::new(r"sk_live_[a-zA-Z0-9]{24}")?,
            rule_id: "CSP-S110",
            severity: "CRITICAL",
            base_score: 95,
        },
        // Stripe Test Key (lower severity)
        BuiltinPattern {
            name: "Stripe Test Key",
            regex: Regex::new(r"sk_test_[a-zA-Z0-9]{24}")?,
            rule_id: "CSP-S111",
            severity: "MEDIUM",
            base_score: 50,
        },
        // SendGrid API Key
        BuiltinPattern {
            name: "SendGrid API Key",
            regex: Regex::new(r"SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43}")?,
            rule_id: "CSP-S115",
            severity: "HIGH",
            base_score: 90,
        },
        // Twilio API Key
        BuiltinPattern {
            name: "Twilio API Key",
            regex: Regex::new(r"SK[a-f0-9]{32}")?,
            rule_id: "CSP-S116",
            severity: "HIGH",
            base_score: 90,
        },
        // Discord Token
        BuiltinPattern {
            name: "Discord Token",
            regex: Regex::new(r"[MN][A-Za-z\d]{23,}\.[\w-]{6}\.[\w-]{27}")?,
            rule_id: "CSP-S119",
            severity: "HIGH",
            base_score: 90,
        },
    ])
}

fn get_generic_patterns() -> Result<Vec<BuiltinPattern>, regex::Error> {
    Ok(vec![
        // Generic API Key
        BuiltinPattern {
            name: "Generic API Key",
            regex: Regex::new(
                r#"(?i)(api_key|apikey|secret|token)\s*=\s*['"][A-Za-z0-9_\-]{20,}['"]"#,
            )?,
            rule_id: "CSP-S103",
            severity: "HIGH",
            base_score: 80,
        },
        // Private Key
        BuiltinPattern {
            name: "Private Key",
            regex: Regex::new(r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----")?,
            rule_id: "CSP-S112",
            severity: "CRITICAL",
            base_score: 95,
        },
        // NPM Token
        BuiltinPattern {
            name: "NPM Token",
            regex: Regex::new(r"npm_[a-zA-Z0-9]{36}")?,
            rule_id: "CSP-S117",
            severity: "HIGH",
            base_score: 90,
        },
        // PyPI Token
        BuiltinPattern {
            name: "PyPI Token",
            regex: Regex::new(r"pypi-[a-zA-Z0-9_-]{50,}")?,
            rule_id: "CSP-S118",
            severity: "HIGH",
            base_score: 90,
        },
        // Database Connection String
        BuiltinPattern {
            name: "Database Connection String",
            regex: Regex::new(r"(?i)(mysql|postgres|mongodb|redis)://[^:]+:[^@]+@[^\s]+")?,
            rule_id: "CSP-S120",
            severity: "CRITICAL",
            base_score: 95,
        },
    ])
}

/// Returns the list of built-in secret patterns.
///
/// Patterns are compiled once and cached for the lifetime of the program.
pub fn get_builtin_patterns() -> &'static Vec<BuiltinPattern> {
    static PATTERNS: OnceLock<Vec<BuiltinPattern>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        try_init_patterns().unwrap_or_else(|e| {
            // This should never happen in production as patterns are verified by tests.
            // But if it does, we return an empty list to avoid crashing the analyzer.
            eprintln!("CRITICAL ERROR: Failed to compile built-in patterns: {e}");
            Vec::new()
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtin_patterns_compile() {
        let patterns = get_builtin_patterns();

        assert!(
            !patterns.is_empty(),
            "Built-in patterns should not be empty"
        );

        for pattern in patterns {
            assert!(!pattern.name.is_empty(), "Pattern name should not be empty");
            assert!(
                !pattern.rule_id.is_empty(),
                "Pattern rule_id should not be empty"
            );
            assert!(
                !pattern.severity.is_empty(),
                "Pattern severity should not be empty"
            );
            assert!(
                pattern.base_score <= 100,
                "Pattern base_score should be <= 100"
            );
            assert!(
                !pattern.regex.as_str().is_empty(),
                "Pattern regex should have content"
            );
        }

        println!("Successfully validated {} builtin patterns", patterns.len());
    }
}
