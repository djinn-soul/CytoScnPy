use std::path::PathBuf;

use crate::config::{CustomSecretPattern, SecretsConfig};

use super::{calculate_entropy, scan_secrets, scan_secrets_compat, validate_secrets_config};

fn default_config() -> SecretsConfig {
    SecretsConfig::default()
}

#[test]
fn test_scanner_detects_github_token() {
    let config = default_config();
    let content = "token = 'ghp_abcdefghijklmnopqrstuvwxyz1234567890'";
    let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None, false);

    assert!(!findings.is_empty());
    assert!(findings.iter().any(|f| f.rule_id == "CSP-S104"));
}

#[test]
fn test_scanner_detects_suspicious_variable() {
    let config = default_config();
    let content = r#"database_password = "super_secret_123""#;
    let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None, false);

    assert!(!findings.is_empty());
    assert!(findings.iter().any(|f| f.rule_id == "CSP-S300"));
}

#[test]
fn test_scanner_skips_env_var() {
    let config = default_config();
    let content = r#"password = os.environ.get("PASSWORD")"#;
    let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None, false);

    assert!(!findings.iter().any(|f| f.rule_id == "CSP-S300"));
}

#[test]
fn test_scanner_entropy_fallback_when_ast_parse_fails() {
    let config = default_config();
    let content = r#"if True print("aB3xY7mN9pQ2rS5tU8vW0zK4cF6gH1jL")"#;
    let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None, false);

    assert!(findings.iter().any(|f| f.rule_id == "CSP-S200"));
}

#[test]
fn test_scanner_reduces_score_in_test_file() {
    let config = default_config();
    let content = r#"api_key = "test_secret_value_12345""#;

    let normal_findings =
        scan_secrets(content, &PathBuf::from("src/main.py"), &config, None, false);
    let test_findings = scan_secrets(
        content,
        &PathBuf::from("tests/test_main.py"),
        &config,
        None,
        true,
    );

    if !normal_findings.is_empty() && !test_findings.is_empty() {
        let normal_conf = normal_findings[0].confidence;
        let test_conf = test_findings[0].confidence;
        assert!(test_conf < normal_conf);
    }
}

#[test]
fn test_scanner_deduplicates_findings() {
    let config = default_config();
    let content = r#"api_key = "ghp_abcdefghijklmnopqrstuvwxyz1234567890""#;
    let findings = scan_secrets(content, &PathBuf::from("test.py"), &config, None, false);

    let line_1_findings: Vec<_> = findings.iter().filter(|f| f.line == 1).collect();
    assert_eq!(line_1_findings.len(), 1);
}

#[test]
fn test_entropy_calculation() {
    assert!(calculate_entropy("aaaaaaaaaa") < 1.0);
    assert!(calculate_entropy("aB3xY7mN9p") > 3.0);
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
    let secrets_config = SecretsConfig {
        patterns: vec![CustomSecretPattern {
            name: "Invalid Regex".to_owned(),
            regex: "[".to_owned(),
            rule_id: None,
            severity: "CRITICAL".to_owned(),
        }],
        ..SecretsConfig::default()
    };

    let config_file = PathBuf::from(".cytoscnpy.toml");
    let findings = validate_secrets_config(&secrets_config, &config_file);

    assert!(!findings.is_empty());
    assert_eq!(findings[0].rule_id, crate::constants::RULE_ID_CONFIG_ERROR);
    assert_eq!(findings[0].file, config_file);
}
