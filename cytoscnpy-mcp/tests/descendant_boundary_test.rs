//! Directory scans must enforce containment for every discovered source.

#![cfg(unix)]
#![allow(clippy::unwrap_used)]

use cytoscnpy_mcp::tools::{AnalyzePathRequest, CytoScnPyServer, MetricsRequest};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use serde_json::Value;
use std::fs;
use std::os::unix::fs::symlink;
use tempfile::tempdir;

fn payload(result: &CallToolResult) -> Value {
    let content = serde_json::to_value(&result.content[0]).unwrap();
    serde_json::from_str(content["text"].as_str().unwrap()).unwrap()
}

#[test]
fn all_path_tools_reject_external_descendant_symlinks() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("allowed");
    fs::create_dir(&root).unwrap();
    let outside = dir.path().join("outside.py");
    fs::write(&outside, "eval(input())\n").unwrap();
    symlink(&outside, root.join("linked.py")).unwrap();
    fs::write(root.join("app.py"), "print('safe')\n").unwrap();
    let server = CytoScnPyServer::with_root(&root).unwrap();
    let analysis = server
        .analyze_path(Parameters(AnalyzePathRequest {
            path: ".".to_owned(),
            scan_secrets: true,
            scan_danger: true,
            check_quality: true,
        }))
        .unwrap();
    let quick = server
        .quick_scan(Parameters(MetricsRequest {
            path: ".".to_owned(),
        }))
        .unwrap();
    for result in [&analysis, &quick] {
        assert_eq!(result.is_error, Some(true));
        let data = payload(result);
        assert_eq!(data["scan_complete"], false);
        assert!(
            !data["parse_errors"].as_array().unwrap().is_empty(),
            "expected a non-empty value"
        );
        assert!(!data.to_string().contains("Avoid using eval"));
    }
    for result in [
        server
            .cyclomatic_complexity(Parameters(MetricsRequest {
                path: ".".to_owned(),
            }))
            .unwrap(),
        server
            .maintainability_index(Parameters(MetricsRequest {
                path: ".".to_owned(),
            }))
            .unwrap(),
    ] {
        assert_eq!(result.is_error, Some(true));
    }
}

#[test]
fn internal_descendant_symlink_is_allowed_without_duplicate_analysis() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("app.py");
    fs::write(&source, "def unused():\n    return 1\n").unwrap();
    symlink(&source, dir.path().join("linked.py")).unwrap();
    let server = CytoScnPyServer::with_root(dir.path()).unwrap();
    let result = server
        .analyze_path(Parameters(AnalyzePathRequest {
            path: ".".to_owned(),
            scan_secrets: false,
            scan_danger: false,
            check_quality: false,
        }))
        .unwrap();
    let data = payload(&result);
    assert_eq!(result.is_error, Some(false));
    assert_eq!(data["scan_complete"], true);
    assert_eq!(data["analysis_summary"]["total_files"], 1);
    assert_eq!(data["unused_functions"].as_array().unwrap().len(), 1);
}
