# CytoScnPy - High-Performance Python Static Analysis 🦀🐍

[![CI](https://github.com/djinn09/CytoScnPy/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/djinn09/CytoScnPy/actions/workflows/rust-ci.yml)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

A lightning-fast static analysis tool for Python codebases, powered by Rust with hybrid Python integration. Detects dead code, security issues, and code quality problems with extreme speed.

## 🚀 Why CytoScnPy?

- **🔥 Blazing Fast**: 79x faster than pure Python implementations
- **💾 Memory Efficient**: Uses 3-4x less memory
- **🐍 Python Native**: Installable via `pip`, importable in Python code
- **⚡ CLI Ready**: Standalone command-line tool
- **🔍 Comprehensive**: Dead code, secrets, security, quality analysis
- **🎯 Framework Aware**: Understands Flask, Django, FastAPI patterns

## 📦 Installation

```bash
# Install from PyPI (when published)
pip install cytoscnpy

# Or install from source
git clone https://github.com/djinn09/CytoScnPy.git
cd cytoscnpy
pip install maturin
maturin develop -m cytoscnpy/Cargo.toml
```

## 🛠️ Usage

### Command Line

```bash
# Basic analysis
cytoscnpy /path/to/project

# With security and quality checks
cytoscnpy . --secrets --danger --quality

# Enforce quality gates (exit code 1 on failure)
cytoscnpy . --max-complexity 10 --min-mi 50 --fail-on-quality
# Or use specific metric commands
cytoscnpy cc . --fail-threshold 10
cytoscnpy mi . --fail-under 50 --average

# Taint analysis (detect data flow vulnerabilities)
cytoscnpy . --taint

# JSON output for CI/CD
cytoscnpy . --json

# Set confidence threshold
cytoscnpy . --confidence 80

# Include test files
cytoscnpy . --include-tests

# Include Jupyter notebooks
cytoscnpy . --include-ipynb
```

### Python API

```python
import cytoscnpy

# Analyze a project
exit_code = cytoscnpy.run(['--json', '/path/to/project'])
print(f"Analysis complete with exit code: {exit_code}")
```

## ✨ Features

### Dead Code Detection

- Unused functions, classes, methods
- Unused imports and variables
- Unreachable code patterns
- Cross-module reference tracking

### Security Analysis

- Hardcoded secrets (API keys, tokens)
- SQL injection risks
- Command injection patterns
- Dangerous code (`eval`, `exec`, `pickle`)
- **Taint analysis** (track data flow from sources to sinks)

### Code Quality

- Cyclomatic complexity (McCabe)
- Halstead metrics
- Maintainability Index (MI)
- Raw metrics (LOC, SLOC)
- Nesting depth analysis
- **Quality Gates**: Fail build on threshold violation

### Framework Support

- Flask route decorators and request objects
- Django ORM patterns and request handling
- FastAPI endpoints
- Automatic pattern recognition

### Notebook Support

- **`--include-ipynb`**: Analyze Jupyter notebooks
- **`--ipynb-cells`**: Report findings per cell

### Configuration

Create `.cytoscnpy.toml` or use `pyproject.toml`:

```toml
[tool.cytoscnpy]
confidence = 60
exclude_folders = ["venv", ".tox", "build"]
include_tests = false
secrets = true
danger = true
quality = true
```

## 📊 Performance

| Metric | Pure Python | Rust (CytoScnPy) | Improvement    |
| ------ | ----------- | ---------------- | -------------- |
| Time   | 14.22s      | 0.18s            | **79x faster** |
| Memory | ~150MB      | ~40MB            | **3.7x less**  |

## 🏗️ Architecture

```
cytoscnpy/
├── Hybrid Distribution
│   ├── Python Package (pip installable)
│   ├── Python API (import cytoscnpy)
│   └── CLI Tool (cytoscnpy command)
│
├── Rust Core
│   ├── AST Analysis (rustpython-parser)
│   ├── Dead Code Detection
│   ├── Security Scanning
│   └── Parallel Processing (rayon)
│
└── Python Integration
    ├── PyO3 Bindings
    ├── Command Wrapper
    └── Maturin Build System
```

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## 📝 License

Apache-2.0 License - see [License](License) file for details.

## 🔗 Links

- **Documentation**: See `cytoscnpy/README.md` for Rust-specific details
- **Benchmarks**: [BENCHMARK.md](benchmark/BENCHMARK.md)
- **Roadmap**: [ROADMAP.md](ROADMAP.md)
- **Changelog**: [CHANGELOG.md](CHANGELOG.md)
