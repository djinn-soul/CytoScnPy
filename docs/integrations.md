# Integrations

CytoScnPy integrates with your development workflow via editor extensions, a language server, and the Model Context Protocol (MCP).

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

## Zed Extension

The [Zed extension](https://github.com/djinn-soul/CytoScnPy/tree/main/editors/zed)
shows CytoScnPy findings as diagnostics in Python files, with quick fixes that
remove unused code or add `# noqa` comments. It runs the language server
below, which it downloads from the latest GitHub release. Like the VS Code
extension, it analyzes the whole workspace folder when you open a Python file
and each time you save one, so code used from another file is not reported as
unused. Pass `{"analysisMode": "file"}` in `initialization_options` to analyze
each file on its own.

The project's `pyproject.toml` or `.cytoscnpy.toml` decides which checks run.
To add CLI flags in Zed only, or to use a binary you installed yourself, set
them in Zed's `settings.json`:

```json
{
  "lsp": {
    "cytoscnpy": {
      "initialization_options": { "args": ["--secrets", "--danger"] },
      "binary": { "path": "/usr/local/bin/cytoscnpy", "arguments": ["lsp"] }
    }
  }
}
```

See the [extension README](https://github.com/djinn-soul/CytoScnPy/blob/main/editors/zed/README.md)
for troubleshooting.

---

## Language Server (Other Editors)

`cytoscnpy lsp` is a Language Server Protocol server over stdio, so any
editor with an LSP client can show CytoScnPy diagnostics and quick fixes. It
behaves like the Zed extension: analysis of the client's workspace folders on
open and save (or of the file alone with `{"analysisMode": "file"}`, and
whenever the file is outside every workspace folder), configuration from the
project, and extra CLI flags from the `args` initialization option. It is
part of the standalone CLI binary; the Python package's `cytoscnpy` command
cannot run it.

**Neovim (0.11+):**

```lua
vim.lsp.config("cytoscnpy", {
  cmd = { "cytoscnpy", "lsp" },
  filetypes = { "python" },
  root_markers = { "pyproject.toml", ".cytoscnpy.toml", ".git" },
  init_options = { args = { "--secrets", "--danger" } },
})
vim.lsp.enable("cytoscnpy")
```

**Helix** (`languages.toml`; `language-servers` replaces the default list, so
keep the servers you already use):

```toml
[language-server.cytoscnpy]
command = "cytoscnpy"
args = ["lsp"]

[[language]]
name = "python"
language-servers = ["pylsp", "cytoscnpy"]
```

---

## MCP Server (AI Assistants)

Enable AI assistants (Claude, Cursor, Copilot) to use CytoScnPy tools.

> **Note**: HTTP/SSE transport is planned for future releases to enable remote analysis. See [Roadmap](roadmap.md) for details.
>
> **Important**: The MCP server is available in the standalone CLI binary (install script or `cytoscnpy-cli` build). The Python `cytoscnpy` package does not run `mcp-server`.

### GitHub Copilot

The VS Code extension **automatically registers** the MCP server. Just ask Copilot:

> _"Run a security scan on this file"_

### Manual Setup (Claude/Cursor)

If not using VS Code, run the server manually using the standalone CLI:

```bash
cytoscnpy mcp-server
```

**Claude Desktop Config**:

```json
{
  "mcpServers": {
    "cytoscnpy": {
      "command": "cytoscnpy",
      "args": ["mcp-server", "--root", "/absolute/path/to/project"]
    }
  }
}
```

Replace `/absolute/path/to/project` with the directory the server may analyze. Omit `--root` only when the client starts the server from that project directory.

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

### GitHub Action

The easiest way to integrate CytoScnPy into GitHub Actions is via the official action:

```yaml
- name: Run CytoScnPy Analysis
  uses: djinn-soul/CytoScnPy@v1
  with:
    args: "--secrets --danger --quality"
```

**Action Inputs:**

| Input            | Description                              | Default  |
| ---------------- | ---------------------------------------- | -------- |
| `path`           | Path(s) to analyze                       | `.`      |
| `args`           | Additional arguments (e.g., `--secrets`) | ` `      |
| `version`        | Version of `cytoscnpy` to install        | `latest` |
| `python-version` | Version of Python to set up              | `3.13`   |

### Other CI/CD Formats

For detailed setup guides and examples for:

- **GitLab Code Quality**
- **GitHub Actions Annotations**
- **SARIF Security Dashboards**
- **JUnit Test Reports**

See the **[CI/CD Integration Guide](usage.md#cicd-integration)** in our User Guide.
