//! Regression tests for incomplete MCP security scans.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use cytoscnpy_mcp::{requests::MetricsRequest, CytoScnPyServer};
use rmcp::handler::server::wrapper::Parameters;
use serde_json::Value;

fn scan(source: &str) -> (bool, Value) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("app.py"), source).unwrap();
    let server = CytoScnPyServer::with_root(dir.path()).unwrap();
    let result = server
        .quick_scan(Parameters(MetricsRequest { path: ".".into() }))
        .unwrap();
    let content = serde_json::to_value(&result.content[0]).unwrap();
    (
        result.is_error.unwrap_or(false),
        serde_json::from_str(content["text"].as_str().unwrap()).unwrap(),
    )
}

#[test]
fn malformed_python_is_an_incomplete_security_scan() {
    let (is_error, summary) = scan("def broken(:\n");
    assert!(is_error);
    assert_eq!(summary["scan_complete"], false);
    assert_eq!(summary["summary"]["scan_errors"], 1);
    assert!(summary["parse_errors"][0]["error"]
        .as_str()
        .unwrap()
        .contains("parameter"));
    assert!(!summary["recommendation"]
        .as_str()
        .unwrap()
        .contains("No security issues"));
}

#[test]
fn valid_clean_python_is_a_complete_security_scan() {
    let (is_error, summary) = scan("print('hello')\n");
    assert!(!is_error);
    assert_eq!(summary["scan_complete"], true);
    assert_eq!(summary["summary"]["scan_errors"], 0);
    assert!(summary["recommendation"]
        .as_str()
        .unwrap()
        .contains("No security issues"));
}

#[test]
fn incomplete_scan_preserves_findings_from_valid_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("broken.py"), "def broken(:\n").unwrap();
    std::fs::write(dir.path().join("valid.py"), "eval(input())\n").unwrap();
    let server = CytoScnPyServer::with_root(dir.path()).unwrap();
    let result = server
        .quick_scan(Parameters(MetricsRequest { path: ".".into() }))
        .unwrap();
    assert_eq!(result.is_error, Some(true));
    let content = serde_json::to_value(&result.content[0]).unwrap();
    let summary: Value = serde_json::from_str(content["text"].as_str().unwrap()).unwrap();
    assert!(!summary["danger"].as_array().unwrap().is_empty());
    assert_eq!(summary["parse_errors"].as_array().unwrap().len(), 1);
}
