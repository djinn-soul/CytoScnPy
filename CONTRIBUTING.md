# Contributing to CytoScnPy (Rust)

Thank you for your interest in contributing to the Rust implementation of CytoScnPy! This document provides guidelines for setting up your environment and making contributions.

## 🛠️ Prerequisites

- **Rust**: Version 1.70 or higher. Install via [rustup.rs](https://rustup.rs).
- **Cargo**: Comes with Rust.
- **Python**: Version 3.8 or higher (for hybrid packaging).
- **UV or pip**: For Python package management.
- **Maturin**: For building PyO3 extensions.
- **Git**: For version control.

## 🚀 Setup Development Environment

### Option A: Rust CLI Development Only

1. **Fork and Clone:**

   ```bash
   git clone https://github.com/YOUR_USERNAME/cytoscnpy.git
   cd cytoscnpy
   ```

2. **Build the Rust CLI:**

   ```bash
   cargo build
   # Or build just the CLI binary
   cargo build --bin cytoscnpy-cli
   ```

3. **Run Tests:**

   ```bash
   cargo test
   ```

4. **Run CytoScnPy:**
   ```bash
   cargo run --bin cytoscnpy-cli -- /path/to/python/project
   ```

### Option B: Hybrid Python + Rust Development (Recommended)

1. **Fork and Clone:**

   ```bash
   git clone https://github.com/YOUR_USERNAME/cytoscnpy.git
   cd cytoscnpy
   ```

2. **Create Python Virtual Environment:**

   ```bash
   # Using uv (recommended)
   uv venv
   source .venv/bin/activate  # Linux/macOS
   .venv\Scripts\activate     # Windows

   # Or using Python's venv
   python -m venv .venv
   source .venv/bin/activate  # Linux/macOS
   .venv\Scripts\activate     # Windows
   ```

3. **Install Maturin:**

   ```bash
   pip install maturin
   ```

4. **Build and Install in Development Mode:**

   ```bash
   # Build and install the Python package with Rust extension
   maturin develop -m cytoscnpy/Cargo.toml

   # Or with release optimizations
   maturin develop -m cytoscnpy/Cargo.toml --release
   ```

5. **Verify Installation:**

   ```bash
   # Test Python import
   python -c "import cytoscnpy; print('Success!')"

   # Test CLI command
   cytoscnpy --help
   ```

6. **Run Tests:**

   ```bash
   # Rust tests
   cargo test

   # Python integration tests (if available)
   pytest
   ```

## 📂 Project Structure

```
CytoScnPy/
├── Cargo.toml                 # Workspace definition
├── pyproject.toml             # Python package metadata (maturin)
│
├── python/                    # Python wrapper code
│   └── cytoscnpy/
│       ├── __init__.py        # Imports Rust `run` function
│       └── cli.py             # CLI wrapper calling Rust
│
├── cytoscnpy/                 # Rust library with PyO3 bindings
│   ├── Cargo.toml             # Library + cdylib configuration
│   ├── tests/                 # Rust integration tests
│   └── src/
│       ├── lib.rs             # Crate root + #[pymodule]
│       ├── main.rs            # Binary entry point (cytoscnpy-bin)
│       ├── python_bindings.rs # PyO3 implementation (modular)
│       ├── entry_point.rs     # Core CLI logic
│       ├── config.rs          # Configuration (.cytoscnpy.toml)
│       ├── cli.rs             # Command-line argument parsing
│       ├── commands.rs        # Radon-compatible commands
│       ├── output.rs          # Rich CLI output
│       ├── linter.rs          # Rule-based linting engine
│       ├── constants.rs       # Shared constants
│       ├── analyzer/          # Main analysis engine
│       │   ├── mod.rs         # Module exports
│       │   ├── types.rs       # AnalysisResult, ParseError types
│       │   ├── heuristics.rs  # Penalty and heuristic logic
│       │   └── processing.rs  # Core processing methods
│       ├── visitor.rs         # AST traversal
│       ├── framework.rs       # Framework-aware patterns
│       ├── test_utils.rs      # Test file detection
│       ├── utils.rs           # Utilities
│       ├── ipynb.rs           # Jupyter notebook support
│       ├── metrics.rs         # Metrics types
│       ├── complexity.rs      # Cyclomatic complexity
│       ├── halstead.rs        # Halstead metrics
│       ├── raw_metrics.rs     # LOC, SLOC metrics
│       ├── rules/             # Security & quality rules
│       │   ├── mod.rs         # Rules module
│       │   ├── secrets.rs     # Secrets scanning + entropy
│       │   ├── danger.rs      # Dangerous code detection
│       │   ├── danger/        # Danger rule helpers
│       │   └── quality.rs     # Code quality checks
│       └── taint/             # Taint analysis module
│           ├── mod.rs         # Module exports
│           ├── types.rs       # TaintFinding, TaintInfo, VulnType
│           ├── analyzer.rs    # Main taint analyzer
│           ├── sources.rs     # Source detection (input, request.*)
│           ├── sinks.rs       # Sink detection (eval, subprocess, SQL)
│           ├── propagation.rs # Taint state tracking
│           ├── intraprocedural.rs  # Statement-level analysis
│           ├── interprocedural.rs  # Cross-function analysis
│           ├── crossfile.rs   # Cross-module analysis
│           ├── call_graph.rs  # Function call graph
│           └── summaries.rs   # Function summaries
│
├── cytoscnpy-cli/             # Standalone Rust binary (optional)
│   ├── Cargo.toml
│   └── src/
│       └── main.rs            # Calls cytoscnpy::entry_point
│
├── benchmark/                 # 126-item ground truth suite
└── target/                    # Build artifacts (gitignored)
```

## 🔄 Development Workflow

1. **Create a Branch:**

   ```bash
   git checkout -b feature/your-rust-feature
   ```

2. **Make Your Changes:**

   - Follow Rust best practices and idioms.
   - Use `rustfmt` for formatting: `cargo fmt`.
   - Use `clippy` for linting: `cargo clippy`.

3. **Test Your Changes:**

   ```bash
   # Run all tests
   cargo test

   # Check compilation
   cargo check

   # Run on test data
   cargo run -- ../test/sample_repo
   ```

4. **Build Release Version:**

   ```bash
   cargo build --release
   ```

5. **Commit and Push:**

   ```bash
   git add .
   git commit -m "feat: your feature description"
   git push origin feature/your-rust-feature
   ```

6. **Open Pull Request:**
   - Open a PR against the `main` branch.
   - Describe your changes and link to any relevant issues.

## 🧩 VS Code Extension Development

The VS Code extension is located in `editors/vscode/cytoscnpy`. It provides real-time analysis by wrapping the Rust CLI.

### Prerequisites

- **Node.js**: Version 16 or higher.
- **VSCE**: `npm install -g @vscode/vsce` (for packaging).

### Setup

1. **Navigate to the extension directory:**

   ```bash
   cd editors/vscode/cytoscnpy
   ```

2. **Install dependencies:**

   ```bash
   npm install
   ```

3. **Compile:**
   ```bash
   npm run compile
   ```

### Running Locally

1. Open `editors/vscode/cytoscnpy` in VS Code.
2. Press `F5` to launch a new Extension Development Host window.
3. Open a Python file in the new window to test the extension.

### Packaging & Publishing

To create a `.vsix` installer:

```bash
vsce package
```

This will generate `cytoscnpy-0.0.1.vsix`.

**Publishing:**
To publish to the VS Code Marketplace, run `vsce publish` after authentication with `vsce login <publisher>`.

## 🐍 Python Integration (PyO3)

CytoScnPy uses **PyO3** to expose Rust functionality to Python, enabling hybrid distribution. This allows users to either:

- Import as a Python package: `import cytoscnpy`
- Use as a CLI tool: `cytoscnpy --help`

### PyO3 Architecture

The Python integration is modular and lives in two places:

1. **`cytoscnpy/src/python_bindings.rs`** - PyO3 implementation

   - Contains all `#[pyfunction]` decorated functions
   - Handles Python↔Rust type conversions
   - Manages GIL (Global Interpreter Lock)

2. **`cytoscnpy/src/lib.rs`** - Module registration
   - Contains the `#[pymodule]` macro (required by PyO3)
   - Delegates to `python_bindings::register_functions()`

### Adding a New Python Function

To expose a new Rust function to Python:

1. **Add the function in `python_bindings.rs`:**

   ```rust
   #[pyfunction]
   fn analyze_file(py: Python, path: String) -> PyResult<String> {
       py.allow_threads(|| {
           // Your Rust implementation
           Ok(format!("Analyzed: {}", path))
       })
   }
   ```

2. **Register it in `register_functions()`:**

   ```rust
   pub(crate) fn register_functions(_py: Python, m: &PyModule) -> PyResult<()> {
       m.add_function(wrap_pyfunction!(run, m)?)?;
       m.add_function(wrap_pyfunction!(analyze_file, m)?)?; // ← Add this
       Ok(())
   }
   ```

3. **Rebuild and test:**
   ```bash
   maturin develop -m cytoscnpy/Cargo.toml
   python -c "import cytoscnpy; print(cytoscnpy.analyze_file('test.py'))"
   ```

### PyO3 Best Practices

- **Release the GIL**: Use `py.allow_threads(|| ...)` for CPU-intensive work
- **Error Handling**: Convert Rust errors to Python exceptions via `PyErr::new`
- **Type Conversions**: Use PyO3's automatic conversions when possible
- **Documentation**: Add docstrings to `#[pyfunction]` functions

For more details, see the PyO3 documentation at [pyo3.rs](https://pyo3.rs).

## 🎯 Priority Areas for Contribution

See [`ROADMAP.md`](ROADMAP.md) for the detailed roadmap.

**High Priority:**

- **Cross-File Analysis:** Improving cross-module import resolution and dead code detection across files.
- **Variable Scope Tracking:** Better detection of unused variables (currently lowest F1 score in benchmarks).
- **Import Detection:** Improving precision/recall for unused import detection.

**Medium Priority:**

- **Type Inference:** Expanding basic type inference for method misuse detection.
- **Framework Support:** Adding more framework patterns (Celery, SQLAlchemy, Pydantic).
- **Performance:** Optimizing for very large codebases (1M+ lines).

## 🔧 Development Tooling

This project uses several Cargo plugins to maintain code quality, security, and developer productivity.

### Required Tools Installation

```bash
# Install all recommended tools (one-time setup)
cargo install cargo-audit cargo-outdated cargo-watch cargo-deny cargo-machete cargo-nextest

# Additional testing tools (optional but recommended)
cargo install cargo-tarpaulin cargo-mutants cargo-semver-checks
```

### Tool Overview

| Tool                    | Purpose                         | Command                        |
| ----------------------- | ------------------------------- | ------------------------------ |
| **Clippy**              | Linting & code quality          | `cargo lint` or `cargo clippy` |
| **cargo-audit**         | Security vulnerability scanning | `cargo audit`                  |
| **cargo-deny**          | Dependency policy enforcement   | `cargo deny check`             |
| **cargo-outdated**      | Check for outdated dependencies | `cargo outdated`               |
| **cargo-machete**       | Detect unused dependencies      | `cargo machete`                |
| **cargo-nextest**       | Next-gen test runner            | `cargo nextest run`            |
| **cargo-watch**         | Auto-rebuild on file changes    | `cargo watch-check`            |
| **cargo-tarpaulin**     | Code coverage reports           | `cargo coverage`               |
| **cargo-mutants**       | Mutation testing (test quality) | `cargo mutants`                |
| **cargo-semver-checks** | Semver violation detection      | `cargo semver`                 |

### Clippy (Linting)

Configured via `Cargo.toml` workspace lints and `clippy.toml`. Pedantic lints are enabled.

```bash
# Run clippy on all targets
cargo lint

# Auto-fix clippy warnings
cargo lint-fix

# Run clippy directly
cargo clippy --all-targets --all-features
```

### Security & Dependencies

```bash
# Check for vulnerable dependencies (RustSec advisory database)
cargo audit

# Check dependency licenses and policies (uses deny.toml)
cargo deny check

# Check for outdated dependencies
cargo outdated

# Detect unused dependencies in Cargo.toml
cargo machete
```

### Testing

```bash
# Standard test runner
cargo test

# Next-gen test runner (better output, flaky test detection)
cargo nextest run

# List all tests
cargo nextest list
```

### Code Coverage (cargo-tarpaulin)

Generate HTML coverage reports to see which code paths are tested:

```bash
# Generate HTML coverage report (outputs to coverage/tarpaulin-report.html)
cargo coverage

# Or run directly with more options
cargo tarpaulin --out Html --out Lcov --output-dir coverage

# View coverage for specific package
cargo tarpaulin -p cytoscnpy --out Html
```

> [!NOTE] > `cargo-tarpaulin` works best on Linux. On Windows, consider using WSL or `cargo llvm-cov` as an alternative.

### Mutation Testing (cargo-mutants)

Verify your tests actually catch bugs by mutating code and checking if tests fail:

```bash
# Run mutation testing (uses 4 parallel jobs)
cargo mutants

# Run on specific file
cargo mutants -- src/config.rs

# Quick check (fewer mutations)
cargo mutants --in-diff HEAD~1
```

> [!TIP]
> Mutation testing is computationally expensive. Run it on specific files or use `--in-diff` for incremental checks.

### Semver Checking (cargo-semver-checks)

Detect breaking API changes after dependency upgrades:

```bash
# Check for semver violations
cargo semver

# Check baseline against current version
cargo semver-checks check-release
```

### Development Workflow (cargo-watch)

Auto-run commands on file changes:

```bash
# Auto-run cargo check on file save
cargo watch-check

# Auto-run tests on file save
cargo watch-test

# Auto-run clippy on file save
cargo watch-lint
```

### Configuration Files

| File                 | Purpose                                                 |
| -------------------- | ------------------------------------------------------- |
| `Cargo.toml`         | Workspace-wide lint configuration (`[workspace.lints]`) |
| `clippy.toml`        | Advanced Clippy thresholds (complexity, line count)     |
| `deny.toml`          | Dependency policies (licenses, bans, advisories)        |
| `.cargo/config.toml` | Cargo aliases and build settings                        |

### 📋 Tooling TODO

The following tools are recommended but not yet fully integrated:

- [ ] **cargo-flamegraph**: Performance profiling with flamegraphs
  - Install: `cargo install flamegraph`
  - Requires OS-level dependencies (perf on Linux, DTrace on macOS)
  - Usage: `cargo flamegraph` to generate `flamegraph.svg`
- [ ] **cargo-make**: Task runner with workflow support
  - Install: `cargo install cargo-make`
  - Create `Makefile.toml` for complex build workflows
  - Useful for CI/CD pipelines
- [ ] **CI Integration**: Add GitHub Actions workflow for:
  - `cargo audit` on every PR
  - `cargo deny check` for license compliance
  - `cargo machete` to catch unused dependencies
  - `cargo nextest run` for test execution

---

## 📝 Coding Guidelines

- **Formatting:** Always run `cargo fmt` before committing.
- **Linting:** Ensure `cargo clippy` passes without warnings.
- **Error Handling:** Use `anyhow::Result` for application-level errors.
- **Documentation:** Add `///` doc comments for public structs and functions.
- **Tests:** Add unit tests for new logic in the same file or in `tests/`.

## 🧪 Testing

### Rust Unit & Integration Tests

```bash
# Run all tests
cargo test

# Run a specific test
cargo test test_name

# Run with output (for debugging)
cargo test -- --nocapture

# Run tests with logging
RUST_LOG=debug cargo test

# Run tests in parallel
cargo test -- --test-threads=4

# Run with specific features
cargo test --features python-tests  # Requires Python in PATH
```

### Python CLI Wrapper Tests

The Python CLI wrapper (`python/cytoscnpy`) has its own test suite:

```bash
# Install dev dependencies
uv pip install -e ".[dev]"

# Run all Python CLI tests
pytest python/tests/ -v

# Run specific test file
pytest python/tests/test_cli.py -v
pytest python/tests/test_integration.py -v
pytest python/tests/test_json_output.py -v
```

### Python Edge Case Test Suite

The Rust implementation is validated against a comprehensive Python test suite (`test/test_rust_edge_cases.py`) that covers ~100+ edge cases and real-world scenarios. This test suite is **critical for ensuring parity** between the Python and Rust implementations.

#### **Test Suite Overview:**

- **72 tests** across 10+ categories
- **56 project fixtures** (isolated temporary projects)
- **Covers advanced Python patterns** (decorators, async/await, metaclasses, etc.)
- **Tests all CLI flags** (--danger, --quality, --secrets, --taint, --confidence)
- **Parametrized tests** for different confidence thresholds

#### **Running Python Tests:**

**Windows:**

```bash
# From workspace root (e:\Github\cytoscnpy)
cd ..  # Go to cytoscnpy root

# Activate Python environment
.venv\Scripts\activate.bat

# Collect all tests
python -m pytest test/test_rust_edge_cases.py --collect-only -q

# Run all tests
python -m pytest test/test_rust_edge_cases.py -v

# Run specific test category (e.g., decorators)
python -m pytest test/test_rust_edge_cases.py -k "decorator" -v

# Run with output
python -m pytest test/test_rust_edge_cases.py -v -s

# Run tests in parallel (requires pytest-xdist)
python -m pytest test/test_rust_edge_cases.py -n auto -v

# Run with coverage
python -m pytest test/test_rust_edge_cases.py --cov=cytoscnpy_rs --cov-report=html -v
```

**Linux/macOS:**

```bash
# From workspace root
cd ..  # Go to cytoscnpy root

# Activate Python environment
source .venv/bin/activate

# Collect all tests
python -m pytest test/test_rust_edge_cases.py --collect-only -q

# Run all tests
python -m pytest test/test_rust_edge_cases.py -v

# Run specific test category (e.g., decorators)
python -m pytest test/test_rust_edge_cases.py -k "decorator" -v

# Run with output
python -m pytest test/test_rust_edge_cases.py -v -s

# Run tests in parallel (requires pytest-xdist)
python -m pytest test/test_rust_edge_cases.py -n auto -v

# Run with coverage
python -m pytest test/test_rust_edge_cases.py --cov=cytoscnpy_rs --cov-report=html -v
```

#### **Test Categories:**

| Category          | Tests | Features                                          |
| ----------------- | ----- | ------------------------------------------------- |
| Nested Structures | 1     | Deeply nested functions/classes                   |
| Decorators        | 7     | Custom, framework (@route), properties            |
| Imports           | 7     | Aliasing, circular, relative, conditional         |
| OOP               | 11    | Inheritance, mixins, metaclasses, dataclasses     |
| Advanced Python   | 15    | Async/await, generators, walrus operator, match   |
| Code Quality      | 5     | Complexity, nesting, arguments, line count        |
| Security          | 3     | SQL injection, command injection, pickle          |
| Performance       | 2     | Large files (100+ functions), multi-file packages |
| Configuration     | 7     | Confidence thresholds, CLI flags                  |
| Edge Cases        | 8     | Empty files, unicode identifiers, long names      |

#### **Key Test Features:**

- **RustAnalyzerRunner**: Executes Rust CLI with JSON output parsing
- **Isolated Fixtures**: Each test creates temporary project directory
- **No Side Effects**: Auto-cleanup after each test
- **Parametrized**: Tests confidence levels `[0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100]`
- **Framework-Aware**: Tests Flask, FastAPI, Django patterns
- **Flag Combinations**: Tests individual and combined CLI flags

#### **When to Run Python Tests:**

1. **After Major Changes**: Test against comprehensive edge cases
2. **Before Pull Request**: Ensure Rust implementation handles real-world code
3. **After Adding Features**: Validate new analyzer logic
4. **For Parity Checking**: Compare Rust vs Python behavior on same code

#### **Example: Testing a New Decorator Pattern**

If you add support for a new decorator pattern in Rust:

1. Look for existing test in `test_rust_edge_cases.py` for that pattern
2. Run: `python -m pytest test/test_rust_edge_cases.py::test_rust_decorator_patterns -v`
3. If test fails, debug your Rust implementation
4. Ensure all decorator tests pass before committing

**Windows Example:**

```bash
.venv\Scripts\activate.bat
python -m pytest test/test_rust_edge_cases.py::test_rust_decorator_patterns -v
```

**Linux/macOS Example:**

```bash
source .venv/bin/activate
python -m pytest test/test_rust_edge_cases.py::test_rust_decorator_patterns -v
```

#### **Example: Testing Framework Support**

To verify Flask/FastAPI route detection works in Rust:

**Windows:**

```bash
.venv\Scripts\activate.bat
python -m pytest test/test_rust_edge_cases.py::test_rust_framework_decorators -v
```

**Linux/macOS:**

```bash
source .venv/bin/activate
python -m pytest test/test_rust_edge_cases.py::test_rust_framework_decorators -v
```

This will test your Rust analyzer against:

- Flask `@app.route()` decorators
- FastAPI `@api.get()` / `@api.post()` decorators
- Django ORM models
- Before/after request handlers
- Error handlers

#### **Debugging Test Failures:**

**Windows:**

```bash
.venv\Scripts\activate.bat

# Show full error output
python -m pytest test/test_rust_edge_cases.py::test_name -vv

# Show print statements and logs
python -m pytest test/test_rust_edge_cases.py::test_name -v -s

# Run with pytest debugger
python -m pytest test/test_rust_edge_cases.py::test_name --pdb
```

**Linux/macOS:**

```bash
source .venv/bin/activate

# Show full error output
python -m pytest test/test_rust_edge_cases.py::test_name -vv

# Show print statements and logs
python -m pytest test/test_rust_edge_cases.py::test_name -v -s

# Run with pytest debugger
python -m pytest test/test_rust_edge_cases.py::test_name --pdb
```

## 🪝 Pre-Commit Hooks

CytoScnPy provides pre-commit hooks for automated code analysis. These hooks allow users of the library to automatically run security and quality checks before each commit.

### Installation for Users

1. **Install pre-commit:**

   ```bash
   pip install pre-commit
   ```

2. **Add to your `.pre-commit-config.yaml`:**

   ```yaml
   repos:
     - repo: https://github.com/djinn09/CytoScnPy
       rev: v1.0.0 # Use the latest release tag
       hooks:
         - id: cytoscnpy-check
           # Optional: customize arguments
           # args: ['--confidence', '50', '--danger', '--quality']
   ```

3. **Install the hooks:**

   ```bash
   pre-commit install
   ```

4. **Run manually (optional):**

   ```bash
   # Run on all files
   pre-commit run --all-files

   # Run on staged files only
   pre-commit run
   ```

### Available Hooks

| Hook ID              | Description                                       |
| -------------------- | ------------------------------------------------- |
| `cytoscnpy-check`    | Run full CytoScnPy analysis (security + quality)  |
| `cytoscnpy-danger`   | Check for dangerous code patterns only            |
| `cytoscnpy-secrets`  | Scan for hardcoded secrets and credentials        |
| `cytoscnpy-quality`  | Check code quality (complexity, unused code)      |
| `cytoscnpy-security` | Security scan with high confidence threshold (70) |

### Configuration

You can customize hook behavior by passing arguments:

```yaml
repos:
  - repo: https://github.com/djinn09/CytoScnPy
    rev: v1.0.0
    hooks:
      - id: cytoscnpy-check
        args:
          - "--confidence"
          - "70" # Only report high-confidence findings
          - "--danger" # Enable dangerous code detection
          - "--quality" # Enable code quality checks
          - "--secrets" # Enable secrets scanning
```

### For Library Maintainers

If you want to add or modify the pre-commit hooks, edit `.pre-commit-hooks.yaml` in the repository root. See [pre-commit.com](https://pre-commit.com/#creating-new-hooks) for the full specification.

---

## ❓ Getting Help

If you have questions, feel free to open an issue with the `question` label or start a discussion on GitHub.

### Testing-Specific Questions

- **How do I add a new test?** See `test/test_rust_edge_cases.py` for patterns and fixtures.
- **Why are tests skipped?** Tests skip if Rust binary (`cytoscnpy`) is not in PATH.
- **Can I test without the Rust binary?** Yes—tests skip gracefully, but you can still run Rust's `cargo test`.
- **How do I validate parity?** Run both `cargo test` and `python -m pytest test/test_rust_edge_cases.py` and compare results.
