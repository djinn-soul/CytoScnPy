# CytoScnPy for VS Code

**CytoScnPy** is a high-performance Python static analyzer written in Rust. This extension integrates CytoScnPy directly into VS Code, providing real-time analysis, security scanning, and code quality metrics.

## Features

- **Real-time Analysis**: Automatically scans your Python files for unused code, security vulnerabilities, and quality issues as you type or save.
- **Security Scanning**: Detects hardcoded secrets (API keys, tokens), SQL injection risks, and dangerous code patterns (`eval`, `exec`).
- **Quality Metrics**: Calculates Cyclomatic Complexity, Halstead Metrics, and Maintainability Index.
- **Inline Diagnostics**: View errors and warnings directly in your editor with detailed hover information.

## Requirements

This extension requires the `cytoscnpy` CLI tool to be available.

### Option 1: Bundled Binary (Default)

The extension comes with a pre-compiled binary for Windows (`cytoscnpy-cli-win32.exe`). If you are on Windows, it should work out of the box.

### Option 2: Python Package (Recommended for Linux/macOS)

For other platforms, or to use a specific version, install the Python package:

```bash
pip install cytoscnpy
```

Then, configure the extension to use the installed executable if it's not automatically detected.

## Extension Settings

This extension contributes the following settings:

| Setting                         | Default     | Description                                                                      |
| :------------------------------ | :---------- | :------------------------------------------------------------------------------- |
| `cytoscnpy.path`                | `cytoscnpy` | Path to the `cytoscnpy` executable.                                              |
| `cytoscnpy.enableSecretsScan`   | `false`     | Enable scanning for hardcoded secrets.                                           |
| `cytoscnpy.enableDangerScan`    | `false`     | Enable scanning for dangerous code patterns.                                     |
| `cytoscnpy.enableQualityScan`   | `false`     | Enable scanning for code quality issues.                                         |
| `cytoscnpy.confidenceThreshold` | `all`       | Minimum confidence level of findings to report (`low`, `medium`, `high`, `all`). |

## Commands

Access these commands from the Command Palette (`Ctrl+Shift+P`):

- **CytoScnPy: Analyze Current File**: Manually trigger analysis for the active file.
- **CytoScnPy: Calculate Cyclomatic Complexity (cc)**: Show complexity metrics.
- **CytoScnPy: Calculate Halstead Metrics (hal)**: Show Halstead metrics.
- **CytoScnPy: Calculate Maintainability Index (mi)**: Show maintainability index.

## Known Issues

- The bundled binary is currently Windows-only. Linux and macOS users must install `cytoscnpy` via pip or build from source.
