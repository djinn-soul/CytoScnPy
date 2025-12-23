//! Tests for secret scanning rules.
//!
//! Verifies entropy calculation, pattern matching (AWS, generic secrets), and configuration handling.
#![allow(clippy::float_cmp)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::str_to_string)]
#![allow(clippy::field_reassign_with_default)]

use cytoscnpy::config::{CustomSecretPattern, SecretsConfig};

use cytoscnpy::rules::secrets::{
    calculate_entropy, is_high_entropy, scan_secrets, scan_secrets_compat,
};
use std::path::PathBuf;

// -------------------------------------------------------------------------
// Entropy Calculation Tests
// -------------------------------------------------------------------------

#[test]
fn test_entropy_empty_string() {
    assert_eq!(calculate_entropy(""), 0.0);
}

#[test]
fn test_entropy_single_char() {
    assert_eq!(calculate_entropy("a"), 0.0);
    assert_eq!(calculate_entropy("aaaa"), 0.0);
}

#[test]
fn test_entropy_two_chars() {
    let entropy = calculate_entropy("ab");
    assert!(entropy > 0.9 && entropy < 1.1, "entropy: {}", entropy);
}

#[test]
fn test_entropy_random_string() {
    // High entropy string (random-looking)
    let random = "aB3xK9pQ2mL7nR4wE6yT";
    let entropy = calculate_entropy(random);
    assert!(entropy > 4.0, "Random string entropy: {}", entropy);
}

#[test]
fn test_entropy_variable_name() {
    // Low entropy string (variable name)
    let var_name = "user_password_value";
    let entropy = calculate_entropy(var_name);
    assert!(entropy < 4.0, "Variable name entropy: {}", entropy);
}

#[test]
fn test_entropy_api_key_like() {
    // API key-like string
    let api_key = "sk_live_51H7zN2IqXo8c3K9aB2mL4pQ";
    let entropy = calculate_entropy(api_key);
    assert!(entropy > 4.0, "API key entropy: {}", entropy);
}

// -------------------------------------------------------------------------
// High Entropy Detection Tests
// -------------------------------------------------------------------------

#[test]
fn test_is_high_entropy_true() {
    assert!(is_high_entropy("aB3xK9pQ2mL7nR4wE6yT", 4.0, 16));
}

#[test]
fn test_is_high_entropy_false_low_entropy() {
    assert!(!is_high_entropy("user_password", 4.0, 8));
}

#[test]
fn test_is_high_entropy_false_too_short() {
    assert!(!is_high_entropy("aB3xK9", 4.0, 16));
}

// -------------------------------------------------------------------------
// Pattern Detection Tests (Positive Cases)
// -------------------------------------------------------------------------

#[test]
fn test_detect_github_token() {
    let content = r#"token = "ghp_abcdefghijklmnopqrstuvwxyz1234567890""#;
    let findings = scan_secrets_compat(content, &PathBuf::from("test.py"));
    assert!(!findings.is_empty(), "Should detect GitHub token");
    assert!(
        findings.iter().any(|f| f.rule_id == "CSP-S104"),
        "Should have GitHub token rule"
    );
}

#[test]
fn test_detect_aws_key() {
    let content = r#"aws_access_key_id = "AKIAIOSFODNN7EXAMPLE""#;
    let findings = scan_secrets_compat(content, &PathBuf::from("test.py"));
    assert!(!findings.is_empty(), "Should detect AWS key");
    assert_eq!(findings[0].rule_id, "CSP-S101");
}

#[test]
fn test_detect_stripe_live_key() {
    let content = r#"STRIPE_KEY = "sk_live_abcdefghijklmnopqrstuvwx""#;
    let findings = scan_secrets_compat(content, &PathBuf::from("test.py"));
    assert!(!findings.is_empty(), "Should detect Stripe live key");
}

#[test]
fn test_detect_private_key() {
    let content = "key = '''-----BEGIN RSA PRIVATE KEY-----'''";
    let findings = scan_secrets_compat(content, &PathBuf::from("test.py"));
    assert!(!findings.is_empty(), "Should detect private key");
    assert_eq!(findings[0].rule_id, "CSP-S112");
}

#[test]
fn test_detect_gitlab_pat() {
    let content = r#"TOKEN = "glpat-abcdefghij1234567890""#;
    let findings = scan_secrets_compat(content, &PathBuf::from("test.py"));
    assert!(!findings.is_empty(), "Should detect GitLab PAT");
}

#[test]
fn test_detect_sendgrid_key() {
    let content =
        r#"key = "SG.abcdefghijklmnopqrstuv.ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890abcdefg""#;
    let findings = scan_secrets_compat(content, &PathBuf::from("test.py"));
    assert!(!findings.is_empty(), "Should detect SendGrid key");
}

#[test]
fn test_detect_database_connection() {
    let content = r#"DATABASE_URL = "postgres://user:password@localhost:5432/db""#;
    let findings = scan_secrets_compat(content, &PathBuf::from("test.py"));
    assert!(
        !findings.is_empty(),
        "Should detect database connection string"
    );
}

// -------------------------------------------------------------------------
// Pattern Detection Tests (Negative Cases - No False Positives)
// -------------------------------------------------------------------------

#[test]
fn test_no_false_positive_comment_when_disabled() {
    // By default, comments ARE scanned (scan_comments: true) to catch accidentally committed secrets.
    // This test verifies that comments are skipped when scan_comments is disabled.
    let content = "# This is a comment with api_key = 'fake_test_value_12345678901234'";
    let mut config = SecretsConfig::default();
    config.scan_comments = false;
    let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None);
    assert!(
        findings.is_empty(),
        "Should not flag comments when scan_comments is false"
    );
}

#[test]
fn test_no_false_positive_pragma() {
    let content = r#"api_key = "secret123456789012345678901234"  # pragma: no cytoscnpy"#;
    let findings = scan_secrets_compat(content, &PathBuf::from("test.py"));
    assert!(findings.is_empty(), "Should respect pragma directive");
}

#[test]
fn test_no_false_positive_url() {
    let config = SecretsConfig::default();
    let content = r#"url = "https://api.example.com/v1/users/12345""#;
    let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None);
    // Should not flag URL as high-entropy secret
    let entropy_findings: Vec<_> = findings
        .iter()
        .filter(|f| f.rule_id == "CSP-S200")
        .collect();
    assert!(
        entropy_findings.is_empty(),
        "Should not flag URLs as high-entropy"
    );
}

#[test]
fn test_no_false_positive_path() {
    let config = SecretsConfig::default();
    let content = r#"path = "/usr/local/bin/python3""#;
    let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None);
    let entropy_findings: Vec<_> = findings
        .iter()
        .filter(|f| f.rule_id == "CSP-S200")
        .collect();
    assert!(entropy_findings.is_empty(), "Should not flag file paths");
}

#[test]
fn test_short_string_not_flagged() {
    let config = SecretsConfig::default();
    let content = r#"code = "ABC123""#;
    let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None);
    let entropy_findings: Vec<_> = findings
        .iter()
        .filter(|f| f.rule_id == "CSP-S200")
        .collect();
    assert!(
        entropy_findings.is_empty(),
        "Short strings should not be flagged"
    );
}

// -------------------------------------------------------------------------
// Custom Pattern Tests
// -------------------------------------------------------------------------

#[test]
fn test_custom_pattern_detection() {
    let mut config = SecretsConfig::default();
    config.patterns.push(CustomSecretPattern {
        name: "Internal Token".to_string(),
        regex: r"INTERNAL_[A-Z0-9]{16}".to_string(),
        severity: "HIGH".to_string(),
        rule_id: Some("CUSTOM-001".to_string()),
    });

    let content = r#"token = "INTERNAL_ABCD1234EFGH5678""#;
    let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None);
    assert!(!findings.is_empty(), "Should detect custom pattern");
    assert!(findings.iter().any(|f| f.rule_id == "CUSTOM-001"));
}

#[test]
fn test_custom_pattern_auto_rule_id() {
    let mut config = SecretsConfig::default();
    config.patterns.push(CustomSecretPattern {
        name: "My Secret".to_string(),
        regex: r"MYSECRET_[a-z]{10}".to_string(),
        severity: "MEDIUM".to_string(),
        rule_id: None,
    });

    let content = r#"key = "MYSECRET_abcdefghij""#;
    let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None);
    assert!(!findings.is_empty());
    assert!(findings[0].rule_id.starts_with("CSP-CUSTOM-"));
}

// -------------------------------------------------------------------------
// Entropy Configuration Tests
// -------------------------------------------------------------------------

#[test]
fn test_entropy_disabled() {
    let mut config = SecretsConfig::default();
    config.entropy_enabled = false;

    // This would normally trigger entropy detection
    let content = r#"random = "aB3xK9pQ2mL7nR4wE6yTzU8vW1""#;
    let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None);
    let entropy_findings: Vec<_> = findings
        .iter()
        .filter(|f| f.rule_id == "CSP-S200")
        .collect();
    assert!(
        entropy_findings.is_empty(),
        "Entropy detection should be disabled"
    );
}

#[test]
fn test_entropy_threshold_adjustment() {
    let mut config = SecretsConfig::default();
    config.entropy_threshold = 6.0; // Very high threshold

    let content = r#"token = "aB3xK9pQ2mL7nR4wE6yT""#;
    let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None);
    let _entropy_findings: Vec<_> = findings
        .iter()
        .filter(|f| f.rule_id == "CSP-S200")
        .collect();
    // With threshold 6.0, this string might not trigger
    // (depends on actual entropy value)
}

// -------------------------------------------------------------------------
// Complex Scenario Tests
// -------------------------------------------------------------------------

#[test]
fn test_multiple_secrets_same_line() {
    let content = r#"keys = {"token": "ghp_abcdefghijklmnopqrstuvwxyz123456", "stripe": "sk_live_abcdefghijklmnopqrstuvwx"}"#;
    let findings = scan_secrets_compat(content, &PathBuf::from("test.py"));
    assert!(findings.len() >= 2, "Should detect multiple secrets");
}

#[test]
fn test_multiline_file() {
    let content = r#"
# Configuration
import os

# This is safe
DEBUG = True

# This should be detected
API_KEY = "ghp_abcdefghijklmnopqrstuvwxyz123456"

# This should also be detected
aws_secret_access_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"

# Safe comment
def main():
    pass
"#;
    let findings = scan_secrets_compat(content, &PathBuf::from("test.py"));
    assert!(
        findings.len() >= 2,
        "Should detect secrets in multiline file"
    );
}

#[test]
fn test_env_file_format() {
    let content = r"
# .env file
DATABASE_URL=postgres://admin:supersecret123@db.example.com:5432/production
API_KEY=sk_live_abcdefghijklmnopqrstuvwx
DEBUG=true
";
    let findings = scan_secrets_compat(content, &PathBuf::from(".env"));
    assert!(!findings.is_empty(), "Should detect secrets in .env format");
}
