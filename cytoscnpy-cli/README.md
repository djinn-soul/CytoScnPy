# CytoScnPy CLI

Standalone command-line binary for CytoScnPy with integrated MCP server.

## Purpose

This is a thin wrapper around the `cytoscnpy` library crate that provides a standalone binary entry point. It also includes the MCP server as a subcommand for AI assistant integrations.

## Usage

```bash
# Build
cargo build --release --package cytoscnpy-cli

# Run dead code analysis
cargo run --package cytoscnpy-cli -- /path/to/project

# Run with all security checks
cargo run --package cytoscnpy-cli -- /path/to/project --secrets --danger

# JSON output for CI/CD
cargo run --package cytoscnpy-cli -- /path/to/project --json

# Metric subcommands
cargo run --package cytoscnpy-cli -- cc /path/to/project  # Cyclomatic complexity
cargo run --package cytoscnpy-cli -- mi /path/to/project  # Maintainability index

# Start MCP server (for AI assistants like Claude, Copilot)
cargo run --package cytoscnpy-cli -- mcp-server
```

## MCP Server

The `mcp-server` subcommand starts an MCP (Model Context Protocol) server over stdio, enabling AI assistants to use CytoScnPy's analysis capabilities:

```bash
cytoscnpy-cli mcp-server
```

See [../cytoscnpy-mcp/README.md](../cytoscnpy-mcp/README.md) for MCP configuration details.

## Structure

- `src/main.rs` - Binary entry point with CLI and MCP server integration
- Depends on `../cytoscnpy` library for core analysis functionality
- Depends on `../cytoscnpy-mcp` library for MCP server functionality

## Note

For Python users, use the `cytoscnpy` command installed via `pip` or `maturin develop` instead of building this binary directly.

See [../README.md](../README.md) for full usage documentation.
