//! Integration tests for analyze_code secret detection.

use cytoscnpy::analyzer::CytoScnPy;
use std::path::PathBuf;

#[test]
fn test_analyze_code_secrets() {
    let analyzer = CytoScnPy::default().with_secrets(true);
    // Using aws_access_key_id which is a built-in pattern
    let code = r#"
    def connect():
        aws_access_key_id = "AKIAIOSFODNN7EXAMPLE"
        password = "correct-horse-battery-staple"
    "#;

    let result = analyzer.analyze_code(code, PathBuf::from("test_secrets.py"));

    // Debug print to see what we found if it fails
    if result.secrets.is_empty() {
        println!("No secrets found!");
    } else {
        for s in &result.secrets {
            println!("Found secret: {} ({})", s.rule_id, s.message);
        }
    }

    assert!(
        !result.secrets.is_empty(),
        "Should detect secrets in analyze_code"
    );
    assert!(
        result.secrets.iter().any(|s| s.rule_id == "CSP-S101"),
        "Should detect AWS key (CSP-S101)"
    );
}
