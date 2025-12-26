//! Command-line interface entry point for `CytoScnPy`.

use anyhow::Result;
use std::process::ExitCode;

use cytoscnpy::entry_point;
use cytoscnpy_mcp::tools::CytoScnPyServer;
use rmcp::ServiceExt;
use tokio::io::{stdin, stdout};

fn main() -> ExitCode {
    // Parse CLI args to check for mcp-server subcommand
    let args: Vec<String> = std::env::args().collect();

    // Check if mcp-server subcommand is being invoked
    if args.len() > 1 && args[1] == "mcp-server" {
        // Start MCP server with tokio runtime
        match run_mcp() {
            Ok(()) => return ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("MCP server error: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    // Delegate all other commands to shared entry_point function
    // Note: We avoid std::process::exit() to allow LLVM profile data flush for PGO builds
    match entry_point::run_with_args(std::env::args().skip(1).collect()) {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_mcp() -> Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(run_mcp_server())
}

/// Run the MCP server for LLM integration.
async fn run_mcp_server() -> Result<()> {
    let server = CytoScnPyServer::new();
    let transport = (stdin(), stdout());
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}
