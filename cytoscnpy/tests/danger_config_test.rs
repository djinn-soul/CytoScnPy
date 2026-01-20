//! Integration tests for danger configuration and severity thresholding.
#![allow(clippy::expect_used)]

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::config::Config;
use std::path::Path;

#[test]
fn test_excluded_rules() {
    let code = "eval('import os')"; // Triggers CSP-D001 (Exec/Eval)
    let mut config = Config::default();
    config.cytoscnpy.danger = Some(true);
    config.cytoscnpy.danger_config.excluded_rules = Some(vec!["CSP-D001".to_owned()]);
    config.cytoscnpy.danger_config.enable_taint = Some(false); // Disable taint to catch raw findings

    let analyzer = CytoScnPy::default().with_danger(true).with_config(config);

    let result = analyzer.analyze_code(code, Path::new("test.py"));
    assert!(result.danger.iter().all(|f| f.rule_id != "CSP-D001"));
}

#[test]
fn test_severity_threshold() {
    let code = "eval('import os')"; // HIGH severity
    let mut config = Config::default();
    config.cytoscnpy.danger = Some(true);
    config.cytoscnpy.danger_config.severity_threshold = Some("CRITICAL".to_owned());
    config.cytoscnpy.danger_config.enable_taint = Some(false); // Disable taint

    let analyzer = CytoScnPy::default().with_danger(true).with_config(config);

    let result = analyzer.analyze_code(code, Path::new("test.py"));
    // CSP-D001 is HIGH, but threshold is CRITICAL, so it should be filtered out
    assert!(result.danger.is_empty());

    // Now test with LOW threshold
    let mut config_low = Config::default();
    config_low.cytoscnpy.danger = Some(true);
    config_low.cytoscnpy.danger_config.severity_threshold = Some("LOW".to_owned());
    config_low.cytoscnpy.danger_config.enable_taint = Some(false); // Disable taint
    let analyzer_low = CytoScnPy::default()
        .with_danger(true)
        .with_config(config_low);
    let result_low = analyzer_low.analyze_code(code, Path::new("test.py"));
    assert!(!result_low.danger.is_empty());
}

#[test]
fn test_custom_sources_taint() {
    let code = "
data = my_custom_source()
eval(data)
";
    let mut config = Config::default();
    config.cytoscnpy.danger = Some(true);
    config.cytoscnpy.danger_config.enable_taint = Some(true);
    config.cytoscnpy.danger_config.custom_sources = Some(vec!["my_custom_source".to_owned()]);

    let analyzer = CytoScnPy::default().with_danger(true).with_config(config);

    let result = analyzer.analyze_code(code, Path::new("test.py"));

    // Find the eval finding (CSP-D001)
    let eval_finding = result
        .danger
        .iter()
        .find(|f| f.rule_id == "CSP-D001")
        .expect("Should find eval finding");

    // Because eval(data) uses data which is from my_custom_source, it should be CRITICAL or HIGH
    // By default injection + taint upgrades HIGH to CRITICAL
    assert_eq!(eval_finding.severity, "CRITICAL");
}
