# CytoScnPy - High-Performance Python Static Analysis

[![CI](https://github.com/djinn09/CytoScnPy/actions/workflows/test-ci.yml/badge.svg)](https://github.com/djinn09/CytoScnPy/actions/workflows/test-ci.yml)
[![Coverage](https://github.com/djinn09/CytoScnPy/actions/workflows/coverage.yml/badge.svg)](https://github.com/djinn09/CytoScnPy/actions/workflows/coverage.yml)
[![codecov](https://codecov.io/gh/djinn09/CytoScnPy/branch/main/graph/badge.svg)](https://codecov.io/gh/djinn09/CytoScnPy)
[![Security Audit](https://github.com/djinn09/CytoScnPy/actions/workflows/security.yml/badge.svg)](https://github.com/djinn09/CytoScnPy/actions/workflows/security.yml)
[![Docs](https://github.com/djinn09/CytoScnPy/actions/workflows/docs.yml/badge.svg)](https://github.com/djinn09/CytoScnPy/actions/workflows/docs.yml)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Version](https://img.shields.io/badge/version-1.2.5-green.svg)](https://github.com/djinn09/CytoScnPy)

A fast static analysis tool for Python codebases, powered by Rust with hybrid Python integration. Detects dead code, security vulnerabilities (including taint analysis), and code quality issues with extreme speed. Code quality metrics are also provided.

## Why CytoScnPy?

- **Blazing Fast**: Faster in dead code detection.
- **Memory Efficient**: Uses less memory.
- **Comprehensive**: Dead code, secrets, security, taint analysis, quality metrics
- **Framework Aware**: Flask, Django, FastAPI, Pydantic, Azure Functions
- **Benchmarked**: Continuous benchmarking with 135-item ground truth suite

## Installation

**Linux / macOS:**

```bash
# Install
curl -fsSL https://raw.githubusercontent.com/djinn09/CytoScnPy/main/install.sh | bash
```

**Windows (PowerShell):**

```powershell
# Install
irm https://raw.githubusercontent.com/djinn09/CytoScnPy/main/install.ps1 | iex
```

**Via Pip:**

```bash
pip install cytoscnpy
```

**From Source:**

```bash
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
  - **Cascading Detection**: Methods inside unused classes are automatically flagged as unused.
  - **Auto-Fix**: Remove dead code automatically with `--fix` (preview by default, use `--apply` to execute).
- **Clone Detection**: Find duplicate code with `--clones`.
- **Security Analysis**: Taint analysis (SQLi, XSS), secret scanning (API keys, suspicious variables), and dangerous code patterns (`eval`, `exec`).
- **Code Quality Metrics**: Cyclomatic complexity, Halstead metrics, Maintainability Index, and raw metrics (LOC, SLOC).
- **Framework Support**: Native understanding of Flask, Django, FastAPI, Pydantic, and Azure Functions v2 patterns.
- **Smart Heuristics**: Handles dataclasses, `__all__` exports, visitor patterns, and dynamic attributes intelligently.
- **Cross-File Detection**: Tracks symbol usage across the entire codebase, including nested packages and complex import chains, to ensure code used in other modules is never incorrectly flagged.

## Usage

> [!IMPORTANT] **Behavioral Change**: Starting from version 1.2.2, tests are **excluded by default** across both the CLI and the library API to reduce noise in production analysis. Use the `--include-tests` flag or set `include_tests = true` in your configuration to scan test files.

### Command Line

```bash
cytoscnpy [PATHS]... [OPTIONS]
```

**Examples:**

```bash
# Dead code analysis
cytoscnpy .                                     # Analyze current directory
cytoscnpy /path/to/project --json               # JSON output for CI/CD

# Security checks (short flags: -s, -d, -q)
cytoscnpy . --secrets --danger --quality
cytoscnpy . -s -d -q                        # Same with short flags

# Confidence threshold (0-100)
cytoscnpy . --confidence 80

# Path filtering
cytoscnpy . --exclude-folder venv --exclude-folder build
cytoscnpy . --include-folder specific_venv      # Override defaults
cytoscnpy . --include-tests

# Jupyter notebooks
cytoscnpy . --include-ipynb --ipynb-cells

# Clone detection (find duplicate code)
cytoscnpy . --clones --clone-similarity 0.8

# Auto-fix dead code (preview first, then apply)
cytoscnpy . --fix                    # Preview changes (dry-run by default)
cytoscnpy . --fix --apply            # Apply changes
cytoscnpy . --fix -a                 # Apply changes (short flag)

# Generate HTML report (quality auto-enabled; add --secrets --danger for security)
cytoscnpy . --html --secrets --danger

# Pre-commit integration
# See https://djinn09.github.io/CytoScnPy/pre-commit/ for setup
```

**Common Options:**

| Flag            | Description                                     |
| --------------- | ----------------------------------------------- |
| `-s, --secrets` | Scan for API keys and hardcoded credentials     |
| `-d, --danger`  | Scan for dangerous code + taint analysis        |
| `-q, --quality` | Scan for code quality issues (complexity, etc.) |
| `--clones`      | Activate duplicate code detection               |
| `--fix`         | Preview/dry-run dead code removal               |
| `-a, --apply`   | Apply fixes to files (use with `--fix`)         |
| `--json`        | Output results in machine-readable JSON         |

> [!TIP] > **[View the Full CLI Reference](docs/CLI.md)** for detailed usage, advanced configuration, and quality gate options.

**CI/CD Gate Options:**

| Flag                   | Description                                |
| ---------------------- | ------------------------------------------ |
| `--fail-threshold <N>` | Exit code 1 if unused code % > N           |
| `--max-complexity <N>` | Exit code 1 if any function complexity > N |
| `--min-mi <N>`         | Exit code 1 if maintainability index < N   |
| `--fail-on-quality`    | Exit code 1 if any quality issues found    |
| `--max-nesting <N>`    | Exit code 1 if any block nesting > N       |
| `--max-args <N>`       | Exit code 1 if any function has > N args   |
| `--max-lines <N>`      | Exit code 1 if any function has > N lines  |

> **Full CLI Reference:** See [docs/CLI.md](docs/CLI.md) for complete command documentation.

### Metric Subcommands

```bash
cytoscnpy raw .                    # Raw Metrics (LOC, SLOC, Comments)
cytoscnpy cc .                     # Cyclomatic Complexity
cytoscnpy hal .                    # Halstead Metrics
cytoscnpy mi .                     # Maintainability Index
cytoscnpy stats . --all            # Full project report (secrets, danger, quality)
cytoscnpy stats . --all -o report.md  # Save report to file
cytoscnpy files .                  # Per-file metrics table
```

> **Tip**: Add `--json` for machine-readable output, `--exclude-folder <DIR>` to skip directories globally, or `--ignore <PATTERN>` for subcommand-specific glob filtering.

## ⚙️ Configuration

Create `.cytoscnpy.toml` (uses `[cytoscnpy]`) or add to `pyproject.toml` (uses `[tool.cytoscnpy]`):

**`.cytoscnpy.toml` example:**

```toml
[cytoscnpy]
# General Settings
confidence = 60  # Minimum confidence threshold (0-100)
exclude_folders = ["venv", ".tox", "build", "node_modules", ".git"]
include_folders = ["src", "tests"]  # Optional: whitelist folders
include_tests = false  # Note: include_ipynb and ipynb_cells are CLI-only (use flags)

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
max_complexity = 10   # Max cyclomatic complexity
max_nesting = 4       # Max indentation depth
min_mi = 65.0         # Minimum Maintainability Index
ignore = ["R001"]     # Ignore specific rule IDs

# Advanced Secret Scanning
[cytoscnpy.secrets_config]
entropy_enabled = true
entropy_threshold = 4.5  # Higher = more random (API keys usually >4.0)
min_length = 16          # Min length to check for entropy
scan_comments = true     # Scan comments for secrets
skip_docstrings = false  # Skip docstrings in entropy scanning
min_score = 50           # Minimum confidence score (0-100)
suspicious_names = ["db_config", "oauth_token"] # Add custom suspicious variable names

# Custom Secret Patterns
[[cytoscnpy.secrets_config.patterns]]
name = "Slack Token"
regex = "xox[baprs]-([0-9a-zA-Z]{10,48})"
severity = "HIGH"
```

> **Note**: Notebook options (`include_ipynb`, `ipynb_cells`) are currently CLI-only but will be added to the configuration file in a future release.

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

### Accuracy (Benchmark Suite: 135 items)

| Detection Type | Precision | Recall   | F1 Score |
| -------------- | --------- | -------- | -------- |
| Classes        | 0.73      | 0.79     | **0.76** |
| Functions      | 0.71      | 0.74     | **0.73** |
| Methods        | **0.86**  | **0.93** | **0.89** |
| Imports        | 0.67      | 0.40     | 0.50     |
| Variables      | 0.30      | 0.15     | 0.20     |
| **Overall**    | **0.71**  | **0.64** | **0.68** |

> See [benchmark/README.md](benchmark/README.md) for detailed comparison against Vulture, Flake8, Pylint, Ruff, and others.

## Testing

See [CONTRIBUTING.md](CONTRIBUTING.md#testing) for testing instructions.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

Apache-2.0 License - see [License](License) file for details.

## Links

- **Documentation**: [CytoScnPy](https://djinn09.github.io/CytoScnPy/)
- **PyPI**: [PyPi](https://pypi.org/project/cytoscnpy/)
- **VS Code Extension**: [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=djinn09.cytoscnpy)
- **Roadmap**: [docs/roadmap.md](docs/roadmap.md)

## References

CytoScnPy's design and implementation are inspired by:

- [**Skylos**](https://github.com/duriantaco/skylos)
- [**Radon**](https://github.com/rubik/radon)
