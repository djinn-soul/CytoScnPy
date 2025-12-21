# CytoScnPy for VS Code

**CytoScnPy** is a high-performance Python static analyzer written in Rust. This extension integrates CytoScnPy directly into VS Code, providing real-time analysis, security scanning, and code quality metrics.

## Features

- **Real-time Analysis**: Automatically scans your Python files for unused code, security vulnerabilities, and quality issues as you type or save.
- **Security Scanning**: Detects hardcoded secrets (API keys, tokens), SQL injection risks, and dangerous code patterns (`eval`, `exec`).
- **Taint Analysis**: Tracks data flow from untrusted sources to dangerous sinks to detect SQL injection, command injection, and code execution vulnerabilities.
- **Quality Metrics**: Calculates Cyclomatic Complexity, Halstead Metrics, and Maintainability Index.
- **Inline Diagnostics**: View errors and warnings directly in your editor with detailed hover information.
- **Workspace Analysis**: Analyze entire directories or workspaces at once.

## Requirements

This extension requires the `cytoscnpy` CLI tool to be available.

### Option 1: Bundled Binary (Default)

The extension comes with pre-compiled binaries for:

- **Windows**: `cytoscnpy-cli-win32.exe`
- **Linux**: `cytoscnpy-cli-linux-x64`
- **macOS**: `cytoscnpy-cli-darwin` (x64) and `cytoscnpy-cli-darwin-arm64` (Apple Silicon)

The appropriate binary is automatically selected based on your platform.

### Option 2: Python Package (Fallback)

If the bundled binary is not available, install the Python package:

```bash
pip install cytoscnpy
```

## Extension Settings

This extension contributes the following settings:

| Setting                             | Default | Description                                                   |
| :---------------------------------- | :------ | :------------------------------------------------------------ |
| `cytoscnpy.path`                    | `""`    | Custom path to the `cytoscnpy` executable (optional).         |
| `cytoscnpy.enableSecretsScan`       | `false` | Enable scanning for hardcoded secrets.                        |
| `cytoscnpy.enableDangerScan`        | `false` | Enable scanning for dangerous code patterns.                  |
| `cytoscnpy.enableQualityScan`       | `false` | Enable scanning for code quality issues.                      |
| `cytoscnpy.confidenceThreshold`     | `75`    | Minimum confidence level (0-100) for reporting findings.      |
| `cytoscnpy.excludeFolders`          | `[]`    | Folders to exclude from analysis (e.g., `["build", "dist"]`). |
| `cytoscnpy.includeFolders`          | `[]`    | Folders to force-include in analysis (e.g., `["tests"]`).     |
| `cytoscnpy.includeTests`            | `false` | Include test files in analysis.                               |
| `cytoscnpy.includeIpynb`            | `false` | Include Jupyter notebooks in analysis.                        |
| `cytoscnpy.maxComplexity`           | `10`    | Maximum allowed cyclomatic complexity before warning.         |
| `cytoscnpy.minMaintainabilityIndex` | `40`    | Minimum maintainability index before warning.                 |
| `cytoscnpy.maxNesting`              | `3`     | Maximum allowed nesting depth before warning.                 |
| `cytoscnpy.maxArguments`            | `5`     | Maximum function arguments before warning.                    |
| `cytoscnpy.maxLines`                | `50`    | Maximum function lines before warning.                        |

## Commands

Access these commands from the Command Palette (`Ctrl+Shift+P` / `Cmd+Shift+P`):

| Command                                        | Description                                    |
| :--------------------------------------------- | :--------------------------------------------- |
| **CytoScnPy: Analyze Current File**            | Trigger analysis for the active Python file.   |
| **CytoScnPy: Analyze Workspace**               | Analyze all Python files in the workspace.     |
| **CytoScnPy: Calculate Raw Metrics**           | Show raw metrics (LOC, SLOC, comments).        |
| **CytoScnPy: Calculate Cyclomatic Complexity** | Show complexity metrics for functions/methods. |
| **CytoScnPy: Calculate Halstead Metrics**      | Show Halstead metrics (volume, difficulty).    |
| **CytoScnPy: Calculate Maintainability Index** | Show maintainability index per function.       |

## GitHub Copilot Integration

This extension automatically registers CytoScnPy as an MCP (Model Context Protocol) server with GitHub Copilot. No manual configuration required!

### Usage

Simply ask Copilot to use CytoScnPy:

- "Run a quick security scan on this file using CytoScnPy"
- "Analyze this code for unused functions with CytoScnPy"
- "Check the cyclomatic complexity of this file"

### Available MCP Tools

| Tool                    | Description                                       |
| :---------------------- | :------------------------------------------------ |
| `analyze_path`          | Full analysis on files/directories                |
| `analyze_code`          | Analyze code snippet directly                     |
| `quick_scan`            | Fast security scan (secrets & dangerous patterns) |
| `cyclomatic_complexity` | Calculate complexity metrics                      |
| `maintainability_index` | Calculate MI scores (0-100)                       |

## Known Issues

- Jupyter notebook support (`.ipynb`) requires the `includeIpynb` setting to be enabled.

## Release Notes

See [CHANGELOG.md](CHANGELOG.md) for detailed release notes.
