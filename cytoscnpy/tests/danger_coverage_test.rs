//! Danger rules coverage tests.
use cytoscnpy::analyzer::CytoScnPy;
use std::path::PathBuf;

#[test]
fn test_danger_rules_full_coverage() {
    let source = include_str!("python_files/danger_corpus.py");
    let mut config = cytoscnpy::config::Config::default();
    config.cytoscnpy.danger_config.enable_taint = Some(false);

    let analyzer = CytoScnPy {
        enable_danger: true,
        config,
        ..CytoScnPy::default()
    };

    // Run analysis
    let result = analyzer.analyze_code(source, &PathBuf::from("python_files/danger_corpus.py"));

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

    // Verify new modern security pattern rules (CSP-D9xx)

    // CSP-D901: Async subprocess
    let async_subprocess_findings: Vec<_> = result
        .danger
        .iter()
        .filter(|f| f.rule_id == "CSP-D004")
        .collect();
    assert!(
        !async_subprocess_findings.is_empty(),
        "Expected async subprocess findings (CSP-D004)"
    );

    // CSP-D902: ML model deserialization
    let model_deser_findings: Vec<_> = result
        .danger
        .iter()
        .filter(|f| f.rule_id == "CSP-D204")
        .collect();
    assert!(
        !model_deser_findings.is_empty(),
        "Expected model deserialization findings (CSP-D204)"
    );

    // CSP-D903: Sensitive data in logs
    let logging_findings: Vec<_> = result
        .danger
        .iter()
        .filter(|f| f.rule_id == "CSP-D901")
        .collect();
    assert!(
        !logging_findings.is_empty(),
        "Expected sensitive data logging findings (CSP-D901)"
    );

    // Assert that expanded SQLi/XSS patterns are found (Comment 1 & 2)
    let sqli_raw_findings: Vec<_> = result
        .danger
        .iter()
        .filter(|f| f.rule_id == "CSP-D102")
        .collect();
    assert!(
        sqli_raw_findings.len() >= 5,
        "Expected at least 5 SQLi raw findings (sqlalchemy, pandas, raw, Template, jinjasql)"
    );

    let xss_findings: Vec<_> = result
        .danger
        .iter()
        .filter(|f| f.rule_id == "CSP-D103")
        .collect();
    assert!(
        xss_findings.len() >= 5,
        "Expected at least 5 XSS findings (flask, jinja2, Markup, format_html, HTMLResponse)"
    );
}

#[test]
fn test_eval_not_filtered_by_taint() {
    let source = "eval('1+1')";
    let mut config = cytoscnpy::config::Config::default();
    config.cytoscnpy.danger_config.enable_taint = Some(true);

    let analyzer = CytoScnPy {
        enable_danger: true,
        config,
        ..CytoScnPy::default()
    };

    let result = analyzer.analyze_code(source, &PathBuf::from("test.py"));

    // Eval should be present even if it's a constant string (not tainted)
    let eval_findings: Vec<_> = result
        .danger
        .iter()
        .filter(|f| f.rule_id == "CSP-D001")
        .collect();
    assert!(!eval_findings.is_empty(), "Eval should always be flagged");
    assert_eq!(eval_findings[0].severity, "HIGH");
}

#[test]
fn test_os_path_taint_detection() {
    let source = "
import os
user_input = input()
os.path.abspath(user_input)
";
    let mut config = cytoscnpy::config::Config::default();
    config.cytoscnpy.danger_config.enable_taint = Some(true);

    let analyzer = CytoScnPy {
        enable_danger: true,
        config,
        ..CytoScnPy::default()
    };

    let result = analyzer.analyze_code(source, &PathBuf::from("test.py"));

    let path_findings: Vec<_> = result
        .danger
        .iter()
        .filter(|f| f.rule_id == "CSP-D501")
        .collect();

    assert!(
        !path_findings.is_empty(),
        "os.path.abspath with tainted input should be flagged"
    );
    // Should be CRITICAL because it's tainted
    assert_eq!(path_findings[0].severity, "CRITICAL");
}

#[test]
fn test_keyword_ssrf_taint_detection() {
    let source = "
import requests
user_url = input()
requests.get(url=user_url)
";
    let mut config = cytoscnpy::config::Config::default();
    config.cytoscnpy.danger_config.enable_taint = Some(true);

    let analyzer = CytoScnPy {
        enable_danger: true,
        config,
        ..CytoScnPy::default()
    };

    let result = analyzer.analyze_code(source, &PathBuf::from("test.py"));

    let ssrf_findings: Vec<_> = result
        .danger
        .iter()
        .filter(|f| f.rule_id == "CSP-D402")
        .collect();

    assert!(
        !ssrf_findings.is_empty(),
        "requests.get(url=...) with tainted input should be flagged"
    );
    assert_eq!(ssrf_findings[0].severity, "CRITICAL");
}

#[test]
fn test_keyword_path_traversal_taint_detection() {
    let source = "
import zipfile
user_path = input()
zipfile.Path('archive.zip', at=user_path)
";
    let mut config = cytoscnpy::config::Config::default();
    config.cytoscnpy.danger_config.enable_taint = Some(true);

    let analyzer = CytoScnPy {
        enable_danger: true,
        config,
        ..CytoScnPy::default()
    };

    let result = analyzer.analyze_code(source, &PathBuf::from("test.py"));

    let path_findings: Vec<_> = result
        .danger
        .iter()
        .filter(|f| f.rule_id == "CSP-D501")
        .collect();

    assert!(
        !path_findings.is_empty(),
        "zipfile.Path(at=...) with tainted input should be flagged"
    );
    assert_eq!(path_findings[0].severity, "CRITICAL");
}

#[test]
fn test_sqli_complex_taint_detection() {
    let source = "
from string import Template
user_sql = input()
Template(user_sql).substitute(id=1) # Should be flagged
";
    let mut config = cytoscnpy::config::Config::default();
    config.cytoscnpy.danger_config.enable_taint = Some(true);

    let analyzer = CytoScnPy {
        enable_danger: true,
        config,
        ..CytoScnPy::default()
    };

    let result = analyzer.analyze_code(source, &PathBuf::from("test.py"));

    let sqli_findings: Vec<_> = result
        .danger
        .iter()
        .filter(|f| f.rule_id == "CSP-D102")
        .collect();

    assert!(
        !sqli_findings.is_empty(),
        "Template.substitute with tainted input should be flagged even with taint filter"
    );
    assert_eq!(sqli_findings[0].severity, "CRITICAL");
}

#[test]
fn test_ssrf_request_taint_detection() {
    let source = "
import requests
user_url = input()
requests.request('GET', user_url)
";
    let mut config = cytoscnpy::config::Config::default();
    config.cytoscnpy.danger_config.enable_taint = Some(true);

    let analyzer = CytoScnPy {
        enable_danger: true,
        config,
        ..CytoScnPy::default()
    };

    let result = analyzer.analyze_code(source, &PathBuf::from("test.py"));

    let ssrf_findings: Vec<_> = result
        .danger
        .iter()
        .filter(|f| f.rule_id == "CSP-D402")
        .collect();

    assert!(
        !ssrf_findings.is_empty(),
        "requests.request('GET', user_url) should be flagged"
    );
    assert_eq!(ssrf_findings[0].severity, "CRITICAL");
}

#[test]
fn test_ssrf_keyword_uri_taint_detection() {
    let source = "
import requests
user_url = input()
requests.get(uri=user_url)
";
    let mut config = cytoscnpy::config::Config::default();
    config.cytoscnpy.danger_config.enable_taint = Some(true);

    let analyzer = CytoScnPy {
        enable_danger: true,
        config,
        ..CytoScnPy::default()
    };

    let result = analyzer.analyze_code(source, &PathBuf::from("test.py"));

    let ssrf_findings: Vec<_> = result
        .danger
        .iter()
        .filter(|f| f.rule_id == "CSP-D402")
        .collect();

    assert!(
        !ssrf_findings.is_empty(),
        "requests.get(uri=user_url) should be flagged"
    );
}

#[test]
fn test_jinjasql_instance_taint_detection() {
    let source = "
from jinjasql import JinjaSql
j = JinjaSql()
user_sql = input()
j.prepare_query(user_sql, {})
";
    let mut config = cytoscnpy::config::Config::default();
    config.cytoscnpy.danger_config.enable_taint = Some(true);

    let analyzer = CytoScnPy {
        enable_danger: true,
        config,
        ..CytoScnPy::default()
    };

    let result = analyzer.analyze_code(source, &PathBuf::from("test.py"));

    let sqli_findings: Vec<_> = result
        .danger
        .iter()
        .filter(|f| f.rule_id == "CSP-D102")
        .collect();

    assert!(
        !sqli_findings.is_empty(),
        "j.prepare_query(user_sql, {{}}) should be flagged"
    );
    assert_eq!(sqli_findings[0].severity, "CRITICAL");
}

#[test]
fn test_os_path_join_all_args_tainted() {
    let source = "
import os
tainted = input()
os.path.join('a', 'b', 'c', tainted)
";
    let mut config = cytoscnpy::config::Config::default();
    config.cytoscnpy.danger_config.enable_taint = Some(true);

    let analyzer = CytoScnPy {
        enable_danger: true,
        config,
        ..CytoScnPy::default()
    };

    let result = analyzer.analyze_code(source, &PathBuf::from("test.py"));

    let path_findings: Vec<_> = result
        .danger
        .iter()
        .filter(|f| f.rule_id == "CSP-D501")
        .collect();

    assert!(
        !path_findings.is_empty(),
        "os.path.join('a', 'b', 'c', tainted) should be flagged"
    );
}
