# Integrations

CytoScnPy integrates with your development workflow via IDE extensions and the Model Context Protocol (MCP).

## VS Code Extension

Get real-time feedback while you code.

### Installation

1. Search for **"CytoScnPy"** in the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=djinn09.cytoscnpy).
2. Install the extension.

### Commands

Access via Command Palette (`Ctrl+Shift+P` / `Cmd+Shift+P`):

| Command                                           | Description                                |
| ------------------------------------------------- | ------------------------------------------ |
| `CytoScnPy: Analyze Current File`                 | Trigger analysis for the active file.      |
| `CytoScnPy: Analyze Workspace`                    | Analyze all Python files in the workspace. |
| `CytoScnPy: Calculate Cyclomatic Complexity (cc)` | Show complexity metrics for current file.  |
| `CytoScnPy: Calculate Halstead Metrics (hal)`     | Show Halstead metrics for current file.    |
| `CytoScnPy: Calculate Maintainability Index (mi)` | Show MI scores for current file.           |
| `CytoScnPy: Calculate Raw Metrics (raw)`          | Show LOC, SLOC, and other raw metrics.     |

### Configuration

Customize behavior in VS Code Settings (`Ctrl+,`):

| `cytoscnpy.analysisMode` | `workspace` | Analysis mode: `workspace` (accurate) or `file` (fast). |
| `cytoscnpy.enableSecretsScan` | `false` | Enable scanning for keys/tokens. |
| `cytoscnpy.enableDangerScan` | `false` | Enable dangerous code patterns scanning. |
| `cytoscnpy.enableQualityScan` | `false` | Enable code quality metrics scanning. |
| `cytoscnpy.enableCloneScan` | `false` | Enable clone detection scanning. |
| `cytoscnpy.confidenceThreshold` | `0` | Min confidence (0-100). 0 shows all findings. |
| `cytoscnpy.excludeFolders` | `[]` | Folders to exclude from analysis. |
| `cytoscnpy.includeFolders` | `[]` | Folders to force-include in analysis. |
| `cytoscnpy.includeTests` | `false` | Include test files in analysis. |
| `cytoscnpy.includeIpynb` | `false` | Include Jupyter Notebooks (.ipynb files). |
| `cytoscnpy.maxComplexity` | `10` | Maximum allowed Cyclomatic Complexity. |
| `cytoscnpy.minMaintainabilityIndex` | `40` | Minimum Maintainability Index. |
| `cytoscnpy.maxNesting` | `3` | Maximum allowed nesting depth. |
| `cytoscnpy.maxArguments` | `5` | Maximum allowed function arguments. |
| `cytoscnpy.maxLines` | `50` | Maximum allowed function lines. |
| `cytoscnpy.path` | `""` | Custom path to CLI executable. |

---

## MCP Server (AI Assistants)

Enable AI assistants (Claude, Cursor, Copilot) to use CytoScnPy tools.

> **Note**: HTTP/SSE transport is planned for future releases to enable remote analysis. See [Roadmap](roadmap.md) for details.

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

---

## CI/CD Integrations

CytoScnPy supports a wide variety of CI/CD environments through structured output formats (JSON, GitLab, SARIF, GitHub Annotations).

For detailed setup guides and examples for:

- **GitLab Code Quality**
- **GitHub Actions Annotations**
- **SARIF Security Dashboards**
- **JUnit Test Reports**

See the **[CI/CD Integration Guide](usage.md#-cicd-integration)** in our User Guide.
