use super::*;
use crate::utils::LineIndex;
use std::path::PathBuf;

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
    let findings = recognizer.scan_text_fallback(content, &PathBuf::from("test.py"));

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

#[test]
fn test_ast_recognizer_does_not_skip_latest_token() {
    let recognizer = AstRecognizer::default();
    let code = r#"latest_token = "super_secret_123""#;

    let parsed = ruff_python_parser::parse_module(code).expect("Failed to parse");
    let line_index = LineIndex::new(code);

    let findings = recognizer.scan_ast(
        &parsed.into_syntax().body,
        &PathBuf::from("test.py"),
        &line_index,
    );

    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("latest_token"));
}

#[test]
fn test_entropy_recognizer_does_not_skip_latest_token_assignment() {
    let recognizer = EntropyRecognizer::default();
    let code = r#"latest_token = "aB3xY7mN9pQ2rS5tU8vW0zK4cF6gH1jL""#;

    let parsed = ruff_python_parser::parse_module(code).expect("Failed to parse");
    let line_index = LineIndex::new(code);
    let findings = recognizer.scan_ast(
        &parsed.into_syntax().body,
        &PathBuf::from("test.py"),
        &line_index,
    );

    assert!(!findings.is_empty());
    assert_eq!(findings[0].rule_id, "CSP-S200");
}

#[test]
fn test_extract_string_literals_with_escapes() {
    let line = r#"val = "string with \" escaped quote" and 'another \' one'"#;
    let literals = EntropyRecognizer::extract_string_literals(line);

    assert_eq!(literals.len(), 2);
    assert_eq!(literals[0], "string with \\\" escaped quote");
    assert_eq!(literals[1], "another \\' one");
}
