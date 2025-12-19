# CytoScnPy - High-Performance Python Static Analysis

[![CI](https://github.com/djinn09/CytoScnPy/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/djinn09/CytoScnPy/actions/workflows/rust-ci.yml)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Version](https://img.shields.io/badge/version-1.1.0-green.svg)](https://github.com/djinn09/CytoScnPy)

A fast static analysis tool for Python codebases, powered by Rust with hybrid Python integration. Detects dead code, security vulnerabilities (including taint analysis), and code quality issues with extreme speed. Code quality metrics are also provided.

## Why CytoScnPy?

- **🔥 Blazing Fast**: Faster in dead code detection.
- **💾 Memory Efficient**: Uses less memory.
- **🔍 Comprehensive**: Dead code, secrets, security, taint analysis, quality metrics
- **🎯 Framework Aware**: Understands Flask, Django, FastAPI patterns
- **📊 Benchmarked**: Continuous benchmarking with 126-item ground truth suite

## Installation

```bash
pip install cytoscnpy

# Or install from source
git clone https://github.com/djinn09/CytoScnPy.git
cd CytoScnPy
pip install maturin
maturin develop -m cytoscnpy/Cargo.toml
```

### MCP Server (for AI Assistants)

CytoScnPy includes an MCP server for AI assistant integration:

```bash
# Start MCP server (after pip install)
cytoscnpy mcp-server
```

For Claude Desktop, Cursor, or GitHub Copilot configuration, see the **[MCP Server Documentation](cytoscnpy-mcp/README.md)**.

## Features

- **Dead Code Detection**: Unused functions, classes, imports, and variables with cross-module tracking.
- **Security Analysis**: Taint analysis (SQLi, XSS), secret scanning (API keys), and dangerous code patterns (`eval`, `exec`).
- **Code Quality Metrics**: Cyclomatic complexity, Halstead metrics, Maintainability Index, and raw metrics (LOC, SLOC).
- **Framework Support**: Native understanding of Flask, Django, and FastAPI patterns.
- **Smart Heuristics**: Handles dataclasses, `__all__` exports, visitor patterns, and dynamic attributes intelligently.
- **Cross-File Detection**: Tracks symbol usage across the entire codebase, including nested packages and complex import chains, to ensure code used in other modules is never incorrectly flagged.

## Usage

### Command Line

```bash
cytoscnpy [PATHS]... [OPTIONS]
```

**Examples:**

```bash
# Dead code analysis
cytoscnpy .                                     # Analyze current directory
cytoscnpy /path/to/project --json               # JSON output for CI/CD

# Security checks (--danger includes taint analysis)
cytoscnpy . --secrets --danger --quality

# Confidence threshold (0-100)
cytoscnpy . --confidence 80

# Path filtering
cytoscnpy . --exclude-folder venv --exclude-folder build
cytoscnpy . --include-folder specific_venv      # Override defaults
cytoscnpy . --include-tests

# Jupyter notebooks
cytoscnpy . --include-ipynb --ipynb-cells
```

**Options:**

| Flag                     | Description                              |
| ------------------------ | ---------------------------------------- |
| `-c, --confidence <N>`   | Set confidence threshold (0-100)         |
| `--secrets`              | Scan for API keys, tokens, credentials   |
| `--danger`               | Scan for dangerous code + taint analysis |
| `--quality`              | Scan for code quality issues             |
| `--json`                 | Output results as JSON                   |
| `-v, --verbose`          | Enable verbose output for debugging      |
| `-q, --quiet`            | Quiet mode: summary only, no tables      |
| `--include-tests`        | Include test files in analysis           |
| `--exclude-folder <DIR>` | Exclude specific folders                 |
| `--include-folder <DIR>` | Force include folders                    |
| `--include-ipynb`        | Include Jupyter notebooks                |
| `--ipynb-cells`          | Report findings per notebook cell        |

**CI/CD Gate Options:**

| Flag                   | Description                                |
| ---------------------- | ------------------------------------------ |
| `--fail-threshold <N>` | Exit code 1 if unused code % > N           |
| `--max-complexity <N>` | Exit code 1 if any function complexity > N |
| `--min-mi <N>`         | Exit code 1 if maintainability index < N   |
| `--fail-on-quality`    | Exit code 1 if any quality issues found    |

> **Full CLI Reference:** See [docs/CLI.md](docs/CLI.md) for complete command documentation.

### Metric Subcommands

```bash
cytoscnpy raw .                    # Raw Metrics (LOC, SLOC, Comments)
cytoscnpy cc .                     # Cyclomatic Complexity
cytoscnpy hal .                    # Halstead Metrics
cytoscnpy mi .                     # Maintainability Index
```

> **Tip**: Add `--json` for machine-readable output, `--exclude-folder <DIR>` to skip directories.

## ⚙️ Configuration

Create `.cytoscnpy.toml` or add to `pyproject.toml`:

```toml
[tool.cytoscnpy]
# General Settings
confidence = 60  # Minimum confidence threshold (0-100)
exclude_folders = ["venv", ".tox", "build", "node_modules", ".git"]
include_folders = ["src", "tests"]  # Optional: whitelist folders
include_tests = false

# Analysis Features
secrets = true
danger = true
quality = true

# Fail Threshold (exit code 1 if exceeded)
fail_threshold = 10.0  # Fail if >10% of code is unused
# fail_threshold = 0.0  # Zero tolerance: fail on any unused code

# Code Quality Thresholds
max_lines = 100       # Max lines per function
max_args = 5          # Max arguments per function
complexity = 10       # Max cyclomatic complexity
nesting = 4           # Max indentation depth
min_mi = 65.0         # Minimum Maintainability Index
ignore = ["R001"]     # Ignore specific rule IDs

# CI/CD Integration
fail_threshold = 5.0  # Exit with code 1 if unused code % exceeds this

# Advanced Secret Scanning
[tool.cytoscnpy.secrets_config]
entropy_enabled = true
entropy_threshold = 4.0  # Higher = more random (API keys usually > 4.0)
min_length = 16          # Min length to check for entropy
scan_comments = true     # Scan comments for secrets

# Custom Secret Patterns
[[tool.cytoscnpy.secrets_config.patterns]]
name = "Slack Token"
regex = "xox[baprs]-([0-9a-zA-Z]{10,48})"
severity = "HIGH"
```

### CI/CD Quality Gates

Configure quality gates for CI/CD pipelines. Set thresholds and the CLI exits with code `1` if exceeded.

**CLI Flags:**

```bash
# Unused code percentage gate
cytoscnpy . --fail-threshold 5  # Fail if >5% unused

# Complexity gate
cytoscnpy . --max-complexity 10  # Fail if any function >10

# Maintainability Index gate
cytoscnpy . --min-mi 40  # Fail if MI <40

# Quiet mode for clean CI output
cytoscnpy . --fail-threshold 5 --quiet
```

**Priority:** CLI flag > config file > environment variable > default

**Environment Variable:** `CYTOSCNPY_FAIL_THRESHOLD=5.0`

## Performance

### Accuracy (Benchmark Suite: 126 items)

| Detection Type | Precision | Recall   | F1 Score |
| -------------- | --------- | -------- | -------- |
| Classes        | 0.75      | 0.82     | 0.78     |
| Functions      | 0.57      | 0.74     | 0.64     |
| Methods        | **1.00**  | 0.59     | 0.74     |
| Imports        | 0.50      | 0.37     | 0.42     |
| Variables      | 0.25      | 0.16     | 0.19     |
| **Overall**    | **0.67**  | **0.59** | **0.63** |

> See [benchmark/README.md](benchmark/README.md) for detailed comparison against Vulture, Flake8, Pylint, Ruff, and others.

## Architecture

See [cytoscnpy/README.md](cytoscnpy/README.md#architecture) for detailed architecture and technology stack information.

## Testing

See [CONTRIBUTING.md](CONTRIBUTING.md#testing) for testing instructions.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

Apache-2.0 License - see [License](License) file for details.

## Links

- [Rust Core Documentation](cytoscnpy/README.md)
- [Benchmarks & Accuracy](benchmark/README.md)
- [Roadmap](ROADMAP.md)
- [Contributing](CONTRIBUTING.md)

## References

CytoScnPy's design and implementation are inspired by:

- [**Skylos**](https://github.com/duriantaco/skylos)
- [**Radon**](https://github.com/rubik/radon)
