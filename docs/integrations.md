# Integrations

CytoScnPy integrates with your development workflow via IDE extensions and the Model Context Protocol (MCP).

## VS Code Extension

Get real-time feedback while you code.

### Installation

1. Search for **"CytoScnPy"** in the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=djinn09.cytoscnpy).
2. Install the extension.

### Commands

Access via Command Palette (`Ctrl+Shift+P` / `Cmd+Shift+P`):

| Command                                      | Description                                |
| -------------------------------------------- | ------------------------------------------ |
| `CytoScnPy: Analyze Current File`            | Trigger analysis for the active file.      |
| `CytoScnPy: Analyze Workspace`               | Analyze all Python files in the workspace. |
| `CytoScnPy: Calculate Cyclomatic Complexity` | Show complexity metrics (cc).              |
| `CytoScnPy: Calculate Halstead Metrics`      | Show Halstead metrics (hal).               |
| `CytoScnPy: Calculate Maintainability Index` | Show Maintainability Index (mi).           |
| `CytoScnPy: Calculate Raw Metrics`           | Show LOC, SLOC, comments.                  |

### Configuration

Customize behavior in VS Code Settings (`Ctrl+,`):

| Setting                         | Default | Description                          |
| ------------------------------- | ------- | ------------------------------------ |
| `cytoscnpy.enableSecretsScan`   | `false` | Enable scanning for keys/tokens.     |
| `cytoscnpy.enableDangerScan`    | `false` | Enable dangerous code patterns.      |
| `cytoscnpy.enableQualityScan`   | `false` | Enable code quality metrics.         |
| `cytoscnpy.enableCloneScan`     | `false` | Enable clone detection.              |
| `cytoscnpy.confidenceThreshold` | `0`     | Min confidence (0-100). 0 shows all. |
| `cytoscnpy.excludeFolders`      | `[]`    | Exclude folders (e.g. `venv`).       |
| `cytoscnpy.path`                | `""`    | Custom path to CLI executable.       |

---

## MCP Server (AI Assistants)

Enable AI assistants (Claude, Cursor, Copilot) to use CytoScnPy tools.

> **Note**: HTTP/SSE transport is planned for future releases to enable remote analysis. See [ROADMAP.md](../ROADMAP.md) for details.

### GitHub Copilot

The VS Code extension **automatically registers** the MCP server. Just ask Copilot:

> _"Run a security scan on this file"_

### Manual Setup (Claude/Cursor)

If not using VS Code, run the server manually:

```bash
cytoscnpy mcp-server
```

**Claude Desktop Config**:

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

### Available Tools

When connected, CytoScnPy exposes the following tools to Claude/Copilot:

| Tool                    | Description                                                           |
| ----------------------- | --------------------------------------------------------------------- |
| `analyze_path`          | Full analysis on files/directories. (Secrets, Danger, Quality, Taint) |
| `analyze_code`          | Analyze a code snippet directly.                                      |
| `quick_scan`            | Fast security scan (Secrets & Dangerous patterns).                    |
| `cyclomatic_complexity` | Calculate complexity metrics for a path.                              |
| `maintainability_index` | Calculate MI scores (0-100) for a path.                               |

### Configuration

#### Claude Desktop

Add to `claude_desktop_config.json`:

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

#### GitHub Copilot (VS Code)

Automatically enabled when the [VS Code Extension](#vs-code-extension) is installed. No extra config needed.
