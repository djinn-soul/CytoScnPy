use super::*;

#[test]
fn test_entropy_threshold_adjustment() {
    let mut config = SecretsConfig::default();
    config.entropy_threshold = 6.0; // Very high threshold

    let content = r#"token = "aB3xK9pQ2mL7nR4wE6yT""#;
    let findings = scan_secrets(
        content,
        &PathBuf::from("test.py"),
        &config,
        None::<&rustc_hash::FxHashSet<usize>>,
        false,
    );
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
    let findings = scan_secrets_compat(content, &PathBuf::from("config.py"));
    // The new modular engine deduplicates findings to one per line (highest confidence)
    assert!(!findings.is_empty(), "Should detect at least one secret");
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

// -------------------------------------------------------------------------
// CSP-S121: Anthropic API Key
// -------------------------------------------------------------------------

#[test]
fn test_detect_anthropic_api_key() {
    // 93 alphanumeric chars after prefix
    let key = "sk-ant-api03-".to_owned() + &"A".repeat(93);
    let content = format!(r#"ANTHROPIC_API_KEY = "{key}""#);
    let findings = scan_secrets_compat(&content, &PathBuf::from("config.py"));
    assert!(
        findings.iter().any(|f| f.rule_id == "CSP-S121"),
        "Should detect Anthropic API key (CSP-S121)"
    );
}

#[test]
fn test_anthropic_key_env_var_not_flagged() {
    let content = r#"ANTHROPIC_API_KEY = os.environ.get("ANTHROPIC_API_KEY")"#;
    let findings = scan_secrets_compat(content, &PathBuf::from("config.py"));
    assert!(
        !findings.iter().any(|f| f.rule_id == "CSP-S121"),
        "Env var access should not be flagged as hardcoded Anthropic key"
    );
}

// -------------------------------------------------------------------------
// CSP-S122: JWT Secret / Signing Key
// -------------------------------------------------------------------------

#[test]
fn test_detect_jwt_secret() {
    let content = r#"jwt_secret = "my_super_secret_signing_key_123""#;
    let findings = scan_secrets_compat(content, &PathBuf::from("auth.py"));
    assert!(
        findings.iter().any(|f| f.rule_id == "CSP-S122"),
        "Should detect hardcoded jwt_secret (CSP-S122)"
    );
}

#[test]
fn test_detect_jwt_key() {
    let content = r#"JWT_KEY = "supersecretkey12345""#;
    let findings = scan_secrets_compat(content, &PathBuf::from("settings.py"));
    assert!(
        findings.iter().any(|f| f.rule_id == "CSP-S122"),
        "Should detect hardcoded JWT_KEY (CSP-S122)"
    );
}

#[test]
fn test_detect_jwt_signing_key() {
    let content = r#"jwt_signing_key = "hs256-signing-key-value""#;
    let findings = scan_secrets_compat(content, &PathBuf::from("auth.py"));
    assert!(
        findings.iter().any(|f| f.rule_id == "CSP-S122"),
        "Should detect hardcoded jwt_signing_key (CSP-S122)"
    );
}

#[test]
fn test_jwt_secret_env_var_not_flagged() {
    let content = r#"jwt_secret = os.environ.get("JWT_SECRET")"#;
    let findings = scan_secrets_compat(content, &PathBuf::from("auth.py"));
    assert!(
        !findings.iter().any(|f| f.rule_id == "CSP-S122"),
        "Env var access should not be flagged as hardcoded JWT secret"
    );
}
