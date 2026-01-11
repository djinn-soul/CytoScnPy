//! Danger rules coverage tests.
use cytoscnpy::analyzer::CytoScnPy;
use std::path::PathBuf;

#[test]
fn test_danger_rules_full_coverage() {
    let source = include_str!("danger_corpus.py");
    let analyzer = CytoScnPy {
        enable_danger: true,
        ..CytoScnPy::default()
    };

    // Run analysis
    let result = analyzer.analyze_code(source, &PathBuf::from("danger_corpus.py"));

    // Assert that we found findings (we expect many)
    assert!(!result.danger.is_empty(), "Expected danger findings");

    // Optional: Check strictly for specific rules if we want to be thorough
    // But for coverage, just hitting the code paths is enough.
    // We can print findings to see what matched if needed.
    // logic in danger.rs is purely AST visitor based, so complex setup isn't needed.

    // Let's verify at least one specific complex rule: Tarfile
    let tar_findings: Vec<_> = result
        .danger
        .iter()
        .filter(|f| f.rule_id == "CSP-D502")
        .collect();
    assert!(
        !tar_findings.is_empty(),
        "Expected Tarfile extraction findings"
    );
}
