# CytoScnPy

**High-Performance Python Static Analysis Tool Powered by Rust**

[![CI](https://github.com/djinn09/CytoScnPy/actions/workflows/test-ci.yml/badge.svg)](https://github.com/djinn09/CytoScnPy/actions/workflows/test-ci.yml)
[![PyPI](https://img.shields.io/pypi/v/cytoscnpy)](https://pypi.org/project/cytoscnpy/)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

CytoScnPy is a blazing fast static analysis tool for Python codebases. It uses a hybrid Rust/Python architecture to detect dead code, security vulnerabilities, and code quality issues with extreme speed and minimal memory footprint.

## Key Features

- 🚀 **Blazing Fast**: Written in Rust for maximum performance.
- 💀 **Dead Code Detection**: Finds unused functions, classes, methods, imports, and variables.
- 🔒 **Security Scanning**: Detects secrets (API keys), dangerous patterns (eval/exec), and taint analysis.
- 📊 **Code Quality**: Calculates Cyclomatic Complexity, Halstead metrics, and Maintainability Index.
- 🧩 **Deep Integration**: VS Code extension and MCP server for AI assistants.
- 🛠️ **Framework Aware**: Native support for Flask, Django, FastAPI, and Pydantic.

## Deep Integration

- **Hybrid Architecture**: High-performance Rust core (`cytoscnpy`) with Python bindings (`PyO3`).
- **Taint Analysis**: Tracks data flow from untrusted sources to dangerous sinks (SQL, Shell, Code Execution).
- **Clone Detection**: Uses Weisfeiler-Lehman graph hashing for semantic similarity.

---

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
maturin develop -m cytoscnpy/Cargo.toml
```

---

## Quick Start

Analyze your current directory for everything (dead code, security, quality):

```bash
cytoscnpy . --secrets --danger --quality
```

Generate a JSON report for CI/CD:

```bash
cytoscnpy . --json > report.json
```

Preview and fix dead code automatically:

```bash
cytoscnpy . --fix        # Preview changes (dry run)
cytoscnpy . --fix --apply # Apply changes
```

---

## Links

- **Documentation**: [djinn09.github.io/CytoScnPy](https://djinn09.github.io/CytoScnPy/)
- **PyPI**: [pypi.org/project/cytoscnpy](https://pypi.org/project/cytoscnpy/)
- **VS Code Extension**: [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=djinn09.cytoscnpy)
- **GitHub Repository**: [github.com/djinn09/CytoScnPy](https://github.com/djinn09/CytoScnPy/)
