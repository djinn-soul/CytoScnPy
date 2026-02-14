//! Regression tests for `scan_secrets_compat`.

use cytoscnpy::rules::secrets::scan_secrets_compat;
use std::path::PathBuf;

#[test]
fn scan_secrets_compat_applies_test_file_penalty_from_path() {
    let content = r#"token = "ghp_abcdefghijklmnopqrstuvwxyz1234567890""#;

    let prod_findings = scan_secrets_compat(content, &PathBuf::from("src/main.py"));
    let test_findings = scan_secrets_compat(content, &PathBuf::from("tests/test_main.py"));

    assert!(
        !prod_findings.is_empty(),
        "Expected a finding for a non-test path"
    );

    let prod_confidence = prod_findings
        .iter()
        .map(|finding| finding.confidence)
        .max()
        .expect("prod findings should contain at least one confidence score");

    if let Some(test_confidence) = test_findings.iter().map(|finding| finding.confidence).max() {
        assert!(
            test_confidence < prod_confidence,
            "Expected test-path confidence ({test_confidence}) to be lower than prod-path confidence ({prod_confidence})"
        );
    } else {
        // Also valid: the test-file penalty can drop score below the default min_score threshold.
        assert!(
            test_findings.is_empty(),
            "Expected no findings when test-file penalty filters below threshold"
        );
    }
}
