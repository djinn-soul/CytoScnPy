//! Tests for entropy scanning of function signature expressions.
//!
//! Issue: `Stmt::FunctionDef` only visited `f.body` but skipped function
//! signature expressions (defaults, annotations, decorators). Secrets like
//! `def login(token="sk_live_...")` were not detected.

#![allow(clippy::unwrap_used, clippy::needless_raw_string_hashes)]

use cytoscnpy::config::SecretsConfig;
use cytoscnpy::rules::secrets::scan_secrets;
use std::path::PathBuf;

/// Test: Secrets in function parameter defaults should be detected.
#[test]
fn test_entropy_detects_function_default_secrets() {
    let content = r#"
def login(token="sk_live_abcdefghijklmnopqrstuvwx"):
    pass
"#;

    let config = SecretsConfig::default();
    let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None);

    // Should detect Stripe live key in default parameter
    assert!(
        !findings.is_empty(),
        "Should detect secret in function default parameter. Got: {:?}",
        findings
    );

    // Specifically should find Stripe key pattern
    let has_stripe = findings.iter().any(|f| {
        f.rule_id == "CSP-S105" // Stripe key pattern
            || f.message.to_lowercase().contains("stripe")
    });

    assert!(
        has_stripe,
        "Should specifically detect Stripe key pattern. Findings: {:?}",
        findings
    );
}

/// Test: Secrets in keyword-only parameter defaults should be detected.
#[test]
fn test_entropy_detects_kwonly_default_secrets() {
    let content = r#"
def authenticate(*, api_key="ghp_abcdefghijklmnopqrstuvwxyz123456"):
    pass
"#;

    let config = SecretsConfig::default();
    let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None);

    assert!(
        !findings.is_empty(),
        "Should detect secret in keyword-only default parameter. Got: {:?}",
        findings
    );

    // Should find a secret pattern (CSP-S103 generic API key or CSP-S104 GitHub token)
    let has_secret = findings
        .iter()
        .any(|f| f.rule_id == "CSP-S104" || f.rule_id == "CSP-S103");
    assert!(
        has_secret,
        "Should detect API key or GitHub token pattern. Findings: {:?}",
        findings
    );
}

/// Test: Secrets in decorator arguments should be detected.
#[test]
fn test_entropy_detects_decorator_secrets() {
    let content = r#"
@authenticate(token="sk_live_abcdefghijklmnopqrstuvwx")
def protected_endpoint():
    pass
"#;

    let config = SecretsConfig::default();
    let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None);

    assert!(
        !findings.is_empty(),
        "Should detect secret in decorator argument. Got: {:?}",
        findings
    );
}

/// Test: Secrets in positional-only parameter defaults should be detected.
#[test]
fn test_entropy_detects_posonly_default_secrets() {
    let content = r#"
def connect(host, password="wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY", /):
    pass
"#;

    let config = SecretsConfig::default();
    let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None);

    assert!(
        !findings.is_empty(),
        "Should detect secret in positional-only default parameter. Got: {:?}",
        findings
    );
}
