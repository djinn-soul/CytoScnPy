# CytoScnPy - High-Performance Python Static Analysis 🦀🐍

[![CI](https://github.com/djinn09/CytoScnPy/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/djinn09/CytoScnPy/actions/workflows/rust-ci.yml)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Version](https://img.shields.io/badge/version-1.0.0-green.svg)](https://github.com/djinn09/CytoScnPy)

A lightning-fast static analysis tool for Python codebases, powered by Rust with hybrid Python integration. Detects dead code, security vulnerabilities (including taint analysis), and code quality issues with extreme speed. Code quality metrics include cyclomatic complexity, Halstead metrics, maintainability index, nesting depth, and more.

## 🚀 Why CytoScnPy?

- **🔥 Blazing Fast**: Faster in dead code detection.
- **💾 Memory Efficient**: Uses less memory.
- **🐍 Python Native**: Installable via `pip`, importable in Python code
- **⚡ CLI Ready**: Standalone command-line tool with rich output
- **🔍 Comprehensive**: Dead code, secrets, security, taint analysis, quality metrics
- **🎯 Framework Aware**: Understands Flask, Django, FastAPI patterns
- **📊 Benchmarked**: Continuous benchmarking with 126-item ground truth suite

## 📦 Installation

```bash
# Install from PyPI (when published)
pip install cytoscnpy

# Or install from source
git clone https://github.com/djinn09/CytoScnPy.git
cd CytoScnPy
pip install maturin
maturin develop -m cytoscnpy/Cargo.toml
```

## 🛠️ Usage

### Command Line

```bash

# Basic usage
cytoscnpy [PATHS]... [OPTIONS]

# Examples:
cytoscnpy .                                     # Analyze current directory
cytoscnpy /path/to/project --json               # Output as JSON
cytoscnpy . --secrets --danger --quality        # Enable specific checks
cytoscnpy . --taint                             # Enable taint analysis

# Options:
#   -c, --confidence <CONFIDENCE>      Set confidence threshold (0-100)
#       --secrets                      Scan for secrets
#       --danger                       Scan for dangerous code patterns
#       --quality                      Scan for code quality issues
#       --taint                        Enable taint analysis
#       --json                         Output results as JSON
#       --include-tests                Include test files in analysis
#       --exclude-folders <FOLDERS>    Exclude specific folders
#       --include-folders <FOLDERS>    Force include specific folders
#       --include-ipynb                Include Jupyter notebooks
#       --ipynb-cells                  Report findings per cell
#   -h, --help                         Print help
#   -V, --version                      Print version

# Subcommands
# -----------------------------------------------------------------------
# Raw Metrics (LOC, SLOC, Comments)
cytoscnpy raw /path/to/project
cytoscnpy raw . --json --exclude-folder venv

# Cyclomatic Complexity (McCabe)
# Calculates complexity for each function/method
cytoscnpy cc .
cytoscnpy cc /path/to/file.py --json

# Halstead Metrics
# Calculates Difficulty, Effort, Volume, Bugs, Time
cytoscnpy hal .
cytoscnpy hal . --exclude-folder tests

# Maintainability Index
# Combined metric (0-100) indicating code maintainability
# < 65 = Hard to maintain
# > 85 = Easy to maintain
cytoscnpy mi .
cytoscnpy mi . --json

> **Note**: Average Complexity and Maintainability Index are automatically calculated and shown in the summary of the main `cytoscnpy .` command.

# Basic dead code analysis
cytoscnpy /path/to/project

# Enable all security checks
cytoscnpy . --secrets --danger --quality --taint

# Taint analysis (detect SQL injection, command injection, code execution)
cytoscnpy . --taint

# Secret scanning with entropy analysis
cytoscnpy . --secrets

# Dangerous code detection (eval, exec, pickle, subprocess)
cytoscnpy . --danger

# Code quality analysis
cytoscnpy . --quality

# Set confidence threshold (0-100)
cytoscnpy . --confidence 80

# JSON output for CI/CD pipelines
cytoscnpy . --json

# Include/exclude paths
cytoscnpy . --exclude-folder venv --exclude-folder build
cytoscnpy . --include-folder specific_venv  # Override default exclusions
cytoscnpy . --include-tests

# Jupyter notebook support
cytoscnpy . --include-ipynb
cytoscnpy . --include-ipynb --ipynb-cells  # Report per cell
```

### Metric Subcommands

```bash
# Raw metrics (LOC, LLOC, SLOC, Comments, Blanks)
cytoscnpy raw . --json

# Cyclomatic Complexity (McCabe)
cytoscnpy cc . --json

# Halstead Metrics (difficulty, effort, volume)
cytoscnpy hal . --json

# Maintainability Index
cytoscnpy mi . --json
```

## ✨ Features

### Dead Code Detection

- **Unused functions, classes, methods** with cross-module tracking
- **Unused imports and variables** with scope-aware analysis
- **Entry point detection** (`if __name__ == "__main__"`) to prevent false positives
- **Dynamic pattern recognition** (`hasattr`, `getattr`, `globals()`)
- **Pragma support** (`# pragma: no cytoscnpy` to suppress findings)

### Security Analysis

CytoScnPy comes with built-in security features to keep your codebase safe:

- **Taint Analysis**: Tracks untrusted user input to prevent SQL Injection and XSS.
- **Secret Scanning**: Finds hardcoded API keys and credentials using high-entropy checks.
- **Dangerous Code**: Alerts you to unsafe usage of `eval()`, `pickle`, and `subprocess`.

> For deep technical details on how the security engine works, see [cytoscnpy/README.md](cytoscnpy/README.md#security-analysis).

Track data flow from untrusted sources to dangerous sinks:

- **Intraprocedural**: Within single functions
- **Interprocedural**: Across functions in the same file
- **Cross-file**: Across module boundaries
- Detects SQL injection, command injection, code execution vulnerabilities

#### Secret Scanning

- Regex patterns for AWS keys, API tokens, private keys
- **Shannon entropy analysis** to reduce false positives
- Detects high-entropy strings that look like real secrets

#### Dangerous Code Patterns

- `eval()`, `exec()`, `compile()` detection
- `pickle` deserialization warnings
- `subprocess` shell injection risks

### Code Quality Metrics

| Metric                    | Description                                           |
| ------------------------- | ----------------------------------------------------- |
| **Raw Metrics**           | LOC, LLOC, SLOC, Comments, Multi-line strings, Blanks |
| **Cyclomatic Complexity** | Control flow complexity (McCabe)                      |
| **Halstead Metrics**      | Difficulty, Effort, Volume, Bugs, Time                |
| **Maintainability Index** | Combined metric (0-100 scale)                         |
| **Nesting Depth**         | Maximum indentation level analysis                    |

### Framework Support

| Framework   | Detected Patterns                                         |
| ----------- | --------------------------------------------------------- |
| **Flask**   | `@app.route`, `request` object sources, `render_template` |
| **Django**  | `request` handling, ORM patterns, template rendering      |
| **FastAPI** | `@app.get/post/...`, `Request` parameter sources          |

### Smart Heuristics

- **Dataclass fields** automatically marked as used
- **Settings/Config classes** with uppercase variables ignored
- **Visitor pattern methods** (`visit_*`, `leave_*`, `transform_*`) marked as used
- **`__all__` exports** prevent flagging as unused
- **Base class tracking** for inheritance-aware analysis
### Configuration

Create `.cytoscnpy.toml` or add to `pyproject.toml`:

```toml
[tool.cytoscnpy]
confidence = 60
exclude_folders = ["venv", ".tox", "build", "node_modules"]
include_tests = false
secrets = true
danger = true
quality = true
fail_threshold = 10.0 # Fail if >10% of code is unused

# Zero Tolerance Policy
# fail_threshold = 0.0

```toml
[tool.cytoscnpy]
# General Settings
confidence = 60  # Minimum confidence threshold (0-100)
exclude_folders = ["venv", ".tox", "build", "node_modules", ".git"]
include_folders = ["src", "tests"] # Optional: whitelist folders
include_tests = false

# Analysis Features
secrets = true
danger = true
quality = true

# Code Quality Thresholds
max_lines = 100       # Max lines per function
max_args = 5          # Max arguments per function
complexity = 10       # Max cyclomatic complexity
nesting = 4           # Max indentation depth
min_mi = 65.0         # Minimum Maintainability Index
ignore = ["R001"]     # Ignore specific rule IDs

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


### Fail Threshold

You can configure a fail threshold for unused code. If the percentage of unused definitions (functions, classes, etc.) exceeds this threshold, the CLI will exit with code `1`.

- **Default**: `100.0` (never fails unless 100% of code is unused, effectively disabled).
- **Zero Tolerance**: Set to `0.0` to fail if _any_ unused code is found.

This can be set in the config file or via the `CYTOSCNPY_FAIL_THRESHOLD` environment variable:

```bash
# Fail if more than 5% of code is unused
export CYTOSCNPY_FAIL_THRESHOLD=5.0
cytoscnpy .
```

## 📊 Performance

### Speed Comparison

| Metric | Pure Python | Rust (CytoScnPy) | Improvement      |
| ------ | ----------- | ---------------- | ---------------- |
| Time   | 14.22s      | **0.07s**        | **~200x faster** |
| Memory | ~150MB      | **~14MB**        | **~10x less**    |

### Accuracy (Benchmark Suite: 126 items)

| Detection Type | Precision | Recall   | F1 Score |
| -------------- | --------- | -------- | -------- |
| Classes        | 0.75      | 0.82     | 0.78     |
| Functions      | 0.57      | 0.74     | 0.64     |
| Methods        | **1.00**  | 0.59     | 0.74     |
| Imports        | 0.50      | 0.37     | 0.42     |
| Variables      | 0.25      | 0.16     | 0.19     |
| **Overall**    | **0.61**  | **0.57** | **0.59** |

> See [BENCHMARK.md](benchmark/BENCHMARK.md) for detailed comparison against Vulture, Flake8, Pylint, Ruff, and others.

## 🏗️ Architecture

See [cytoscnpy/README.md](cytoscnpy/README.md#architecture) for detailed architecture and technology stack information.

## 🧪 Testing

See [CONTRIBUTING.md](CONTRIBUTING.md#testing) for testing instructions.

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## 📝 License

Apache-2.0 License - see [License](License) file for details.

## 🔗 Links

- **Rust Core Documentation**: [cytoscnpy/README.md](cytoscnpy/README.md)
- **Benchmarks & Accuracy**: [BENCHMARK.md](benchmark/BENCHMARK.md)
- **Roadmap**: [ROADMAP.md](ROADMAP.md)
- **Changelog**: [CHANGELOG.md](CHANGELOG.md)
- **Contributing**: [CONTRIBUTING.md](CONTRIBUTING.md)

## 📚 References

CytoScnPy's design and implementation in Rust are inspired by and reference the following Python libraries:

- [**Skylos**](https://github.com/duriantaco/skylos)
- [**Radon**](https://github.com/rubik/radon)
