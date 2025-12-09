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

### Python API

```python
import cytoscnpy

# Analyze a project
exit_code = cytoscnpy.run(['--json', '/path/to/project'])
print(f"Analysis complete with exit code: {exit_code}")

# Run with multiple flags
exit_code = cytoscnpy.run(['--secrets', '--taint', '--quality', '.'])
```

## ✨ Features

### Dead Code Detection

- **Unused functions, classes, methods** with cross-module tracking
- **Unused imports and variables** with scope-aware analysis
- **Entry point detection** (`if __name__ == "__main__"`) to prevent false positives
- **Dynamic pattern recognition** (`hasattr`, `getattr`, `globals()`)
- **Pragma support** (`# pragma: no cytoscnpy` to suppress findings)

### Security Analysis

#### Taint Analysis (v1.0.0)

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

```
CytoScnPy/
├── cytoscnpy/                    # Rust core library
│   └── src/
│       ├── analyzer/             # Core analysis engine
│       ├── visitor.rs            # AST visitor implementation
│       ├── rules/                # Security & quality rules
│       │   ├── danger.rs         # Dangerous code detection
│       │   ├── secrets.rs        # Secret scanning + entropy
│       │   └── quality.rs        # Code quality checks
│       ├── taint/                # Taint analysis engine
│       │   ├── sources.rs        # User input sources
│       │   ├── sinks.rs          # Dangerous sinks
│       │   ├── intraprocedural.rs
│       │   ├── interprocedural.rs
│       │   └── crossfile.rs
│       ├── complexity.rs         # Cyclomatic complexity
│       ├── halstead.rs           # Halstead metrics
│       ├── raw_metrics.rs        # LOC/SLOC counting
│       └── python_bindings.rs    # PyO3 integration
│
├── cytoscnpy-cli/                # Standalone CLI binary
├── python/                       # Python package wrapper
└── benchmark/                    # 126-item ground truth suite
```

### Technology Stack

| Component           | Technology                                         |
| ------------------- | -------------------------------------------------- |
| **Parser**          | `rustpython-parser` (Python 3.12 compatible)       |
| **Parallelization** | `rayon` for multi-threaded file processing         |
| **CLI**             | `clap` with derive macros                          |
| **Python Bindings** | `PyO3` + `maturin` build system                    |
| **Output**          | `colored` + `comfy-table` for rich terminal output |

## 🧪 Testing

```bash
# Run all tests (119+ tests)
cargo test --workspace

# Run with specific features
cargo test --features python-tests  # Requires Python in PATH
```

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