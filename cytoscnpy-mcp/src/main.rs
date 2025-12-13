//! CytoScnPy MCP Server
//!
//! This binary exposes CytoScnPy's static analysis capabilities as MCP tools,
//! allowing LLMs like Claude to use CytoScnPy for Python code analysis.

mod tools;

use anyhow::Result;
use rmcp::ServiceExt;
use tokio::io::{stdin, stdout};
use tools::CytoScnPyServer;

/// Main entry point for the MCP server.
///
/// Starts the server using stdio transport, which is the standard way
/// for MCP clients like Claude Desktop to communicate with servers.
#[tokio::main]
async fn main() -> Result<()> {
    // Create the server instance
    let server = CytoScnPyServer::new();

    // Start serving on stdio (stdin/stdout)
    // This allows Claude Desktop, Cursor, and other MCP clients to communicate
    let transport = (stdin(), stdout());
    let service = server.serve(transport).await?;

    // Wait for the service to complete
    service.waiting().await?;

    Ok(())
}
