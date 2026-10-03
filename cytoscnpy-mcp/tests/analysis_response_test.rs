//! Full MCP analysis must mark incomplete scans as errors while retaining findings.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use cytoscnpy_mcp::{
    requests::{AnalyzeCodeRequest, AnalyzePathRequest},
    CytoScnPyServer,
};
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult};
use serde_json::Value;

fn payload(result: &CallToolResult) -> Value {
    let content = serde_json::to_value(&result.content[0]).unwrap();
    serde_json::from_str(content["text"].as_str().unwrap()).unwrap()
}

#[test]
fn code_analysis_marks_parse_failures_and_clean_results() {
    let server = CytoScnPyServer::default();
    for (source, complete) in [("def broken(:\n", false), ("print('hello')\n", true)] {
        let result = server
            .analyze_code(Parameters(AnalyzeCodeRequest {
                code: source.into(),
                filename: "snippet.py".into(),
            }))
            .unwrap();
        assert_eq!(result.is_error, Some(!complete));
        let result = payload(&result);
        assert_eq!(result["scan_complete"], complete);
        assert_eq!(
            result["parse_errors"].as_array().unwrap().is_empty(),
            complete
        );
    }
}

#[test]
fn incomplete_path_analysis_retains_findings_from_valid_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("broken.py"), "def broken(:\n").unwrap();
    std::fs::write(dir.path().join("valid.py"), "eval(input())\n").unwrap();
    let server = CytoScnPyServer::with_root(dir.path()).unwrap();
    let result = server
        .analyze_path(Parameters(AnalyzePathRequest {
            path: ".".into(),
            scan_secrets: true,
            scan_danger: true,
            check_quality: true,
        }))
        .unwrap();
    assert_eq!(result.is_error, Some(true));
    let result = payload(&result);
    assert_eq!(result["scan_complete"], false);
    assert_eq!(result["parse_errors"].as_array().unwrap().len(), 1);
    assert!(!result["danger"].as_array().unwrap().is_empty());
}

#[test]
fn readable_valid_path_analysis_is_complete() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("app.py"), "print('hello')\n").unwrap();
    let server = CytoScnPyServer::with_root(dir.path()).unwrap();
    let result = server
        .analyze_path(Parameters(AnalyzePathRequest {
            path: ".".into(),
            scan_secrets: true,
            scan_danger: true,
            check_quality: true,
        }))
        .unwrap();
    assert_eq!(result.is_error, Some(false));
    assert_eq!(payload(&result)["scan_complete"], true);
}
