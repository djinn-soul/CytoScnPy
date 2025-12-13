# CytoScnPy MCP Server

MCP (Model Context Protocol) server that exposes CytoScnPy's Python static analysis capabilities to LLMs like Claude, GPT, and other AI assistants.

## What is MCP?

The [Model Context Protocol](https://modelcontextprotocol.io/) is an open standard that allows AI assistants to use external tools. This server enables LLMs to analyze Python code for:

- **Unused code** - Functions, classes, imports, variables
- **Secrets** - Hardcoded API keys, passwords, tokens
- **Dangerous patterns** - `eval`, `exec`, SQL injection
- **Code quality** - Cyclomatic complexity, maintainability index

## Installation

Build the release binary:

```bash
cargo build --release -p cytoscnpy-mcp
```

The binary will be at `target/release/cytoscnpy-mcp` (or `.exe` on Windows).

## Usage

### Claude Desktop

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "cytoscnpy": {
      "command": "E:/Github/CytoScnPy/target/release/cytoscnpy-mcp.exe"
    }
  }
}
```

Config file location:

- **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`
- **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`

Then restart Claude Desktop and ask: _"Use cytoscnpy to analyze my Python project at /path/to/project"_

### Cursor IDE

Add to Cursor's MCP settings with the path to the binary.

## Available Tools

| Tool                    | Description                        | Parameters                                                               |
| ----------------------- | ---------------------------------- | ------------------------------------------------------------------------ |
| `analyze_path`          | Full analysis on files/directories | `path`, `scan_secrets`, `scan_danger`, `check_quality`, `taint_analysis` |
| `analyze_code`          | Analyze code snippet directly      | `code`, `filename`                                                       |
| `cyclomatic_complexity` | Calculate complexity metrics       | `path`                                                                   |
| `maintainability_index` | Calculate MI scores (0-100)        | `path`                                                                   |

### Example Tool Calls

**Analyze a project:**

```json
{
  "tool": "analyze_path",
  "arguments": {
    "path": "/home/user/myproject",
    "scan_secrets": true,
    "scan_danger": true
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
│  Claude / LLM       │
│  (MCP Client)       │
└──────────┬──────────┘
           │ JSON-RPC over stdio
           ▼
┌─────────────────────┐
│  cytoscnpy-mcp      │
│  (This binary)      │
└──────────┬──────────┘
           │ Direct Rust function calls
           ▼
┌─────────────────────┐
│  cytoscnpy library  │
│  (Core analysis)    │
└─────────────────────┘
```

## Future: HTTP Transport

HTTP/SSE transport is planned for remote LLM integrations. See [ROADMAP.md](../ROADMAP.md) for details.

## License

Apache-2.0
