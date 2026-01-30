# CytoScnPy MCP Server

MCP (Model Context Protocol) server that exposes CytoScnPy's Python static analysis capabilities to LLMs like Claude, GPT, GitHub Copilot, and other AI assistants.

## What is MCP?

The [Model Context Protocol](https://modelcontextprotocol.io/) is an open standard that allows AI assistants to use external tools. This server enables LLMs to analyze Python code for:

- **Unused code** - Functions, classes, imports, variables
- **Secrets** - Hardcoded API keys, passwords, tokens
- **Dangerous patterns** - `eval`, `exec`, SQL injection
- **Code quality** - Cyclomatic complexity, maintainability index

## Installation

### Option 1: Standalone CLI (Recommended)

The MCP server is available in the standalone CLI binary:

```bash
# Install (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/djinn09/CytoScnPy/main/install.sh | bash

# Run MCP server (standalone CLI)
cytoscnpy mcp-server
```

The Python `cytoscnpy` package does not run `mcp-server`.

### Option 2: VS Code Extension (Automatic)

Install the [CytoScnPy VS Code extension](../editors/vscode/cytoscnpy/README.md). It automatically registers the MCP server with GitHub Copilot—no manual configuration required.

### Option 3: Install Script (Standalone Binary)

**Linux / macOS:**

```bash
# Install
curl -fsSL https://raw.githubusercontent.com/djinn09/CytoScnPy/main/install.sh | bash

# Start MCP server
cytoscnpy mcp-server
```

**Windows (PowerShell):**

```powershell
# Install
irm https://raw.githubusercontent.com/djinn09/CytoScnPy/main/install.ps1 | iex

# Start MCP server (after restarting terminal)
cytoscnpy mcp-server
```

### Build from Source

```bash
cargo build --release -p cytoscnpy-cli

# Run MCP server
./target/release/cytoscnpy-cli mcp-server
```

## Usage

### Claude Desktop

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "cytoscnpy": {
      "command": "cytoscnpy",
      "args": ["mcp-server"]
    }
  }
}
```

### Cursor IDE

Add to Cursor's MCP settings:

```json
{
  "cytoscnpy": {
    "command": "cytoscnpy",
    "args": ["mcp-server"]
  }
}
```

### GitHub Copilot (VS Code)

The VS Code extension automatically registers the MCP server. Just install the extension and ask Copilot:

> "Run a quick security scan on this file using CytoScnPy"

## Available Tools

| Tool                    | Description                                       | Parameters                                                               |
| ----------------------- | ------------------------------------------------- | ------------------------------------------------------------------------ |
| `analyze_path`          | Full analysis on files/directories                | `path`, `scan_secrets`, `scan_danger`, `check_quality`, `taint_analysis` |
| `analyze_code`          | Analyze code snippet directly                     | `code`, `filename`                                                       |
| `quick_scan`            | Fast security scan (secrets & dangerous patterns) | `path`                                                                   |
| `cyclomatic_complexity` | Calculate complexity metrics                      | `path`                                                                   |
| `maintainability_index` | Calculate MI scores (0-100)                       | `path`                                                                   |

### Example Tool Calls

**Quick security scan:**

```json
{
  "tool": "quick_scan",
  "arguments": {
    "path": "/home/user/myproject"
  }
}
```

**Full analysis:**

```json
{
  "tool": "analyze_path",
  "arguments": {
    "path": "/home/user/myproject",
    "scan_secrets": true,
    "scan_danger": true,
    "check_quality": true
  }
}
```

**Analyze code snippet:**

```json
{
  "tool": "analyze_code",
  "arguments": {
    "code": "def unused():\n    pass\n\ndef main():\n    print('hello')\n",
    "filename": "example.py"
  }
}
```

## Architecture

```
┌─────────────────────┐
│  Claude / Copilot   │
│  (MCP Client)       │
└──────────┬──────────┘
           │ JSON-RPC over stdio
           ▼
┌─────────────────────┐
│  cytoscnpy          │
│  mcp-server         │
└──────────┬──────────┘
           │ Direct Rust function calls
           ▼
┌─────────────────────┐
│  cytoscnpy library  │
│  (Core analysis)    │
└─────────────────────┘
```

## Future: HTTP Transport

HTTP/SSE transport is planned for remote LLM integrations. See [roadmap.md](../docs/roadmap.md) for details.

## License

Apache-2.0
