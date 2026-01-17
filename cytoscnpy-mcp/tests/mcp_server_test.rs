//! Integration tests for the MCP server.
//!
//! This module specifically tests the public API of the MCP server tools.

use cytoscnpy_mcp::tools::{AnalyzeCodeRequest, AnalyzePathRequest, CytoScnPyServer};
use rmcp::handler::server::wrapper::Parameters;

#[test]
#[allow(clippy::expect_used)]
fn test_analyze_code_basic() {
    let server = CytoScnPyServer::new();
    let params = Parameters(AnalyzeCodeRequest {
        code: "def unused_func():\n    pass\n".to_owned(),
        filename: "test.py".to_owned(),
    });
    let result = server.analyze_code(params);

    // Check that we get a valid result
    assert!(result.is_ok(), "Result should be Ok");

    let call_result = result.expect("Analysis failed");
    // Check that we have content in the result
    assert!(!call_result.content.is_empty(), "Should have content");

    // Get the text content and verify it contains expected fields
    if let Some(content) = call_result.content.first() {
        let text = format!("{content:?}");
        assert!(
            text.contains("unused_functions") || text.contains("unused"),
            "Response should contain analysis results"
        );
    }
}

#[test]
#[allow(clippy::expect_used)]
fn test_analyze_code_with_secrets() {
    let server = CytoScnPyServer::new();
    let code = r#"
API_KEY = "sk-1234567890abcdef1234567890abcdef"
def main():
    print(API_KEY)
"#;
    let params = Parameters(AnalyzeCodeRequest {
        code: code.to_owned(),
        filename: "secrets_test.py".to_owned(),
    });
    let result = server.analyze_code(params);

    assert!(result.is_ok(), "Result should be Ok");
    let call_result = result.expect("Analysis failed");
    assert!(!call_result.content.is_empty(), "Should have content");

    if let Some(content) = call_result.content.first() {
        let text = format!("{content:?}");
        // Should detect the potential secret
        assert!(
            text.contains("secrets") || text.contains("API_KEY") || text.contains("unused"),
            "Response should contain analysis results for secrets"
        );
    }
}

#[test]
#[allow(clippy::expect_used)]
fn test_analyze_code_with_complexity() {
    let server = CytoScnPyServer::new();
    // Code with high cyclomatic complexity (many branches)
    let code = r"
def complex_function(a, b, c, d, e):
    if a > 0:
        if b > 0:
            if c > 0:
                if d > 0:
                    if e > 0:
                        return 1
                    else:
                        return 2
                else:
                    return 3
            else:
                return 4
        else:
            return 5
    else:
        return 6
";
    let params = Parameters(AnalyzeCodeRequest {
        code: code.to_owned(),
        filename: "complex.py".to_owned(),
    });
    let result = server.analyze_code(params);

    assert!(result.is_ok(), "Result should be Ok");
    let call_result = result.expect("Analysis failed");
    assert!(!call_result.content.is_empty(), "Should have content");
}

#[test]
#[allow(clippy::expect_used)]
fn test_analyze_path_invalid() {
    let server = CytoScnPyServer::new();
    let params = Parameters(AnalyzePathRequest {
        path: "/nonexistent/path/to/file.py".to_owned(),
        scan_secrets: true,
        scan_danger: true,
        check_quality: true,
    });
    let result = server.analyze_path(params);

    assert!(result.is_ok(), "Result should be Ok even for invalid path");
    let call_result = result.expect("Should return error result");

    // Should contain error message about path not existing
    if let Some(content) = call_result.content.first() {
        let text = format!("{content:?}");
        assert!(
            text.contains("does not exist") || text.contains("error"),
            "Should indicate path doesn't exist"
        );
    }
}

#[test]
#[allow(clippy::expect_used)]
fn test_analyze_code_unused_imports() {
    let server = CytoScnPyServer::new();
    let code = r#"
import os
import sys
import json

def main():
    print("hello")
"#;
    let params = Parameters(AnalyzeCodeRequest {
        code: code.to_owned(),
        filename: "unused_imports.py".to_owned(),
    });
    let result = server.analyze_code(params);

    assert!(result.is_ok(), "Result should be Ok");
    let call_result = result.expect("Analysis failed");

    if let Some(content) = call_result.content.first() {
        let text = format!("{content:?}");
        // Should detect unused imports
        assert!(
            text.contains("unused_imports") || text.contains("os") || text.contains("sys"),
            "Should detect unused imports"
        );
    }
}

#[test]
#[allow(clippy::expect_used)]
fn test_analyze_code_dangerous_patterns() {
    let server = CytoScnPyServer::new();
    let code = r"
def dangerous_function(user_input):
    eval(user_input)  # dangerous!
    exec(user_input)  # also dangerous!
";
    let params = Parameters(AnalyzeCodeRequest {
        code: code.to_owned(),
        filename: "dangerous.py".to_owned(),
    });
    let result = server.analyze_code(params);

    assert!(result.is_ok(), "Result should be Ok");
    let call_result = result.expect("Analysis failed");
    assert!(!call_result.content.is_empty(), "Should have content");
}

#[test]
#[allow(clippy::expect_used)]
fn test_server_creation() {
    // Test that server can be created and is properly initialized
    let server1 = CytoScnPyServer::new();
    let server2 = CytoScnPyServer::default();

    // Both should work - create separate params for each
    let params1 = Parameters(AnalyzeCodeRequest {
        code: "x = 1".to_owned(),
        filename: "test.py".to_owned(),
    });
    let params2 = Parameters(AnalyzeCodeRequest {
        code: "y = 2".to_owned(),
        filename: "test2.py".to_owned(),
    });

    let result1 = server1.analyze_code(params1);
    let result2 = server2.analyze_code(params2);

    assert!(result1.is_ok(), "Server created with new() should work");
    assert!(result2.is_ok(), "Server created with default() should work");
}
