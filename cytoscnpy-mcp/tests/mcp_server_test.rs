//! Integration tests for the MCP server.
//!
//! This module specifically tests the public API of the MCP server tools.

use cytoscnpy_mcp::tools::{AnalyzeCodeRequest, CytoScnPyServer};

#[test]
fn test_analyze_code_basic() {
    let server = CytoScnPyServer::new();
    let result = server.analyze_code(AnalyzeCodeRequest {
        code: "def unused_func():\n    pass\n".to_owned(),
        filename: "test.py".to_owned(),
    });

    // Check that we get a valid JSON response with expected fields
    assert!(
        result.contains("unused_functions"),
        "Response should contain unused_functions"
    );
    assert!(
        !result.contains("\"error\""),
        "Response should not contain error"
    );
}
