//! Command-line interface entry point for `CytoScnPy`.

use anyhow::Result;

use cytoscnpy::entry_point;
use cytoscnpy_mcp::tools::CytoScnPyServer;
use rmcp::ServiceExt;
use tokio::io::{stdin, stdout};

fn main() -> Result<()> {
    // Parse CLI args to check for mcp-server subcommand
    let args: Vec<String> = std::env::args().collect();

    // Check if mcp-server subcommand is being invoked
    if args.len() > 1 && args[1] == "mcp-server" {
        // Start MCP server with tokio runtime
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(run_mcp_server())?;
        return Ok(());
    }

    // Delegate all other commands to shared entry_point function
    let code = entry_point::run_with_args(std::env::args().skip(1).collect())?;
    std::process::exit(code);
}

/// Run the MCP server for LLM integration.
async fn run_mcp_server() -> Result<()> {
    let server = CytoScnPyServer::new();
    let transport = (stdin(), stdout());
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}
