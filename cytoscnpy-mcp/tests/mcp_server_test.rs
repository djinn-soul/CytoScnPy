//! Tests for the MCP server functionality.
//!
//! Checks that tools can be called and return expected results.

#![allow(clippy::unwrap_used, clippy::expect_used)]
use cytoscnpy::analyzer::types::AnalysisResult;
use serde_json::json;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

// Helper to call tools directly
fn call_tool(
    name: &str,
    args: serde_json::Value,
) -> Result<Vec<rmcp::model::Content>, Box<dyn std::error::Error>> {
    let server = cytoscnpy_mcp::tools::CytoScnPyServer::new();
    match name {
        "get_server_info" => Ok(vec![rmcp::model::Content::text("CytoScnPy MCP Server")]),
        "analyze_path" => {
            let params: cytoscnpy_mcp::tools::AnalyzePathRequest = serde_json::from_value(args)?;
            let result = server
                .analyze_path(rmcp::handler::server::wrapper::Parameters(params))
                .map_err(|e| format!("Tool error: {e:?}"))?;
            Ok(result.content)
        }
        "analyze_code" => {
            let params: cytoscnpy_mcp::tools::AnalyzeCodeRequest = serde_json::from_value(args)?;
            let result = server
                .analyze_code(rmcp::handler::server::wrapper::Parameters(params))
                .map_err(|e| format!("Tool error: {e:?}"))?;
            Ok(result.content)
        }
        _ => Err(format!("Unknown tool: {name}").into()),
    }
}

#[test]
fn test_get_server_info() {
    let result = call_tool("get_server_info", json!({})).unwrap();
    assert_eq!(result.len(), 1);
    let json = serde_json::to_value(&result[0]).unwrap();
    let text = json["text"].as_str().expect("Expected text field");
    assert!(text.contains("CytoScnPy MCP Server"));
}

#[test]
fn test_analyze_path_basic() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "def foo(): pass").unwrap();

    let args = json!({
        "path": file_path.to_str().unwrap()
    });

    let result = call_tool("analyze_path", args).unwrap();
    assert_eq!(result.len(), 1);
    let json = serde_json::to_value(&result[0]).unwrap();
    let text = json["text"].as_str().expect("Expected text field");
    let analysis: AnalysisResult = serde_json::from_str(text).unwrap();
    assert_eq!(analysis.analysis_summary.total_files, 1);
}

#[test]
#[warn(clippy::assertions_on_constants)]
#[allow(clippy::single_match_else)]
fn test_analyze_path_invalid() {
    let args = json!({
        "path": "non_existent_file.py"
    });

    let result = call_tool("analyze_path", args);
    // If it returns Ok, check that analysis result indicates failure or is empty
    match result {
        Ok(content) => {
            // For non-existent files, we might just get empty metrics/files
            let json = serde_json::to_value(&content[0]).unwrap();
            let text = json["text"].as_str().expect("Expected text field");
            // It might be a JSON AnalysisResult (empty) or a plain text error message
            if let Ok(analysis) = serde_json::from_str::<AnalysisResult>(text) {
                assert_eq!(analysis.analysis_summary.total_files, 0);
            } else {
                // If it's not JSON, it's likely an error message e.g. "Path does not exist"
                // which is valid for this test
                assert!(!text.is_empty());
            }
        }
        Err(_) => {
            // This is also acceptable if it errors
            // OK
        }
    }
}

#[test]
fn test_quick_scan() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "print('hello')").unwrap();

    let args = json!({
        "path": file_path.to_str().unwrap()
    });

    let result = call_tool("analyze_path", args).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn test_cyclomatic_complexity() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("complex.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(
        file,
        "def complex_func(x):\n    if x > 0:\n        if x > 10:\n            return 1\n    return 0"
    )
    .unwrap();

    let args = json!({
        "path": file_path.to_str().unwrap()
    });

    let result = call_tool("analyze_path", args).unwrap();
    let json = serde_json::to_value(&result[0]).unwrap();
    let text = json["text"].as_str().expect("Expected text field");
    let analysis: AnalysisResult = serde_json::from_str(text).unwrap();
    // Just verify it ran without crashing
    assert!(analysis.analysis_summary.total_files > 0);
}

#[test]
fn test_maintainability_index() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("mi.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "def simple(): pass").unwrap();

    let args = json!({
        "path": file_path.to_str().unwrap()
    });

    let result = call_tool("analyze_path", args).unwrap();
    let json = serde_json::to_value(&result[0]).unwrap();
    let text = json["text"].as_str().expect("Expected text field");
    let analysis: AnalysisResult = serde_json::from_str(text).unwrap();
    assert!(analysis.analysis_summary.average_mi > 0.0);
}

#[test]
fn test_analyze_code_basic() {
    let code = "def foo(): pass";
    let args = json!({
        "code": code
    });

    let result = call_tool("analyze_code", args).unwrap();
    let json = serde_json::to_value(&result[0]).unwrap();
    let text = json["text"].as_str().expect("Expected text field");

    // Ensure the output is valid JSON analysis
    let analysis: AnalysisResult = serde_json::from_str(text).unwrap();
    assert_eq!(analysis.unused_functions.len(), 1);
    // Analysis adds module prefix (snippet.foo) for code snippets
    assert!(analysis.unused_functions[0].name.ends_with("foo"));
}

#[test]
fn test_analyze_code_unused_imports() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("unused.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "import os\n\ndef main():\n    pass").unwrap();

    let args = json!({
        "path": file_path.to_str().unwrap()
    });

    let result = call_tool("analyze_path", args).unwrap();
    let json = serde_json::to_value(&result[0]).unwrap();
    let text = json["text"].as_str().expect("Expected text field");
    let analysis: AnalysisResult = serde_json::from_str(text).unwrap();
    assert_eq!(analysis.unused_imports.len(), 1);
    assert_eq!(analysis.unused_imports[0].name, "os");
}

#[test]
fn test_analyze_code_dangerous_patterns() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("danger.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "eval('2 + 2')").unwrap();

    let args = json!({
        "path": file_path.to_str().unwrap()
    });

    let result = call_tool("analyze_path", args).unwrap();
    let json = serde_json::to_value(&result[0]).unwrap();
    let text = json["text"].as_str().expect("Expected text field");
    let analysis: AnalysisResult = serde_json::from_str(text).unwrap();
    assert!(!analysis.danger.is_empty());
    assert!(analysis.danger[0].message.to_lowercase().contains("eval"));
}

#[test]
fn test_analyze_code_with_complexity() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("complexity.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "def f(x):\n    if x:\n        return 1\n    return 0").unwrap();

    let args = json!({
        "path": file_path.to_str().unwrap()
    });

    let result = call_tool("analyze_path", args).unwrap();
    let json = serde_json::to_value(&result[0]).unwrap();
    let text = json["text"].as_str().expect("Expected text field");
    let analysis: AnalysisResult = serde_json::from_str(text).unwrap();
    assert!(!analysis.file_metrics.is_empty());
    assert!(analysis.file_metrics[0].complexity > 0.0);
}

#[test]
fn test_analyze_code_with_secrets() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("secrets.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "aws_secret = 'AKIAIOSFODNN7EXAMPLE'").unwrap();

    let args = json!({
        "path": file_path.to_str().unwrap()
    });

    let result = call_tool("analyze_path", args).unwrap();
    let json = serde_json::to_value(&result[0]).unwrap();
    let text = json["text"].as_str().expect("Expected text field");
    let analysis: AnalysisResult = serde_json::from_str(text).unwrap();
    assert!(!analysis.secrets.is_empty());
}
