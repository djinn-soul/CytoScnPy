//! Integration tests for the MCP server.
//!
//! This module specifically tests the public API of the MCP server tools.

use cytoscnpy_mcp::tools::{AnalyzeCodeRequest, CytoScnPyServer};
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
