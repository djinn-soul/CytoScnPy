# Contributing to CytoScnPy (Rust)

Thank you for your interest in contributing to the Rust implementation of CytoScnPy! This document provides guidelines for setting up your environment and making contributions.

## Prerequisites

- **Rust**: Version 1.70 or higher. Install via [rustup.rs](https://rustup.rs).
- **Cargo**: Comes with Rust.
- **Python**: Version 3.8 or higher (for hybrid packaging).
- **UV or pip**: For Python package management.
- **Maturin**: For building PyO3 extensions.
- **Git**: For version control.

## Setup Development Environment

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
   # Using uv (strongly recommended)
   uv venv
   source .venv/bin/activate  # Linux/macOS
   .venv\Scripts\activate     # Windows

   # Or using Python's venv
   python -m venv .venv
   source .venv/bin/activate  # Linux/macOS
   .venv\Scripts\activate     # Windows
   ```

3. **Install Dependencies:**

   ```bash
   # Using uv (fast)
   uv pip install -e ".[dev]"

   # Or using pip
   pip install -e ".[dev]"
   ```

## Developing cytoscnpy-mcp

The MCP server implementation is located in `cytoscnpy-mcp/`. It allows CytoScnPy to be used as a tool by AI assistants.

### Running the MCP Server locally

```bash
cargo run --bin cytoscnpy-mcp
```

### Testing the MCP Server

```bash
# Run MCP-specific tests
cargo test -p cytoscnpy-mcp
```

## Project Structure

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
├── cytoscnpy/                 # Core Rust library with PyO3 bindings
│   ├── Cargo.toml
│   └── src/
│       └── ...
│
├── cytoscnpy-cli/             # Standalone Rust binary
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
│
├── cytoscnpy-mcp/             # MCP Server implementation
│   ├── Cargo.toml
│   ├── src/                   # Rust implementation
│   └── tests/                 # MCP-specific tests
│
├── benchmark/                 # 135-item ground truth suite
└── target/                    # Build artifacts (gitignored)
```

## GitHub Actions Workflows

The project includes several CI/CD workflows in `.github/workflows/`:

| Workflow         | File                        | Trigger         | Purpose                                     |
| ---------------- | --------------------------- | --------------- | ------------------------------------------- |
| **Test Suite**   | `ci.yml`                    | PR to main      | Build, nextest, pytest                      |
| **Benchmark**    | `benchmark.yml`             | PR to main      | Run accuracy benchmarks, detect regressions |
| **Coverage**     | `coverage.yml`              | Push to main    | Generate and upload code coverage reports   |
| **Security**     | `security.yml`              | PR/Push to main | `cargo audit`, `deny`, `machete`            |
| **Publish**      | `publish.yml`               | Git tags (`v*`) | Build wheels and publish to PyPI/TestPyPI   |
| **VS Code Bins** | `vscode-binaries.yml`       | Manual          | Build VS Code extension binaries            |
| **PGO Profiles** | `generate-pgo-profiles.yml` | Manual          | Generate PGO profiling data                 |

### Running Workflows Locally

You can test workflows locally using [act](https://github.com/nektos/act):

```bash
# Install act (requires Docker)
# Test the rust-ci workflow
act -W .github/workflows/rust-ci.yml
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

## VS Code Extension Development

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

## Python Integration (PyO3)

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

## Priority Areas for Contribution

See [`docs/roadmap.md`](docs/roadmap.md) for the detailed roadmap.

**High Priority:**

- **Cross-File Analysis:** Improving cross-module import resolution and dead code detection across files.
- **Variable Scope Tracking:** Better detection of unused variables (currently lowest F1 score in benchmarks).
- **Import Detection:** Improving precision/recall for unused import detection.

**Medium Priority:**

- **Type Inference:** Expanding basic type inference for method misuse detection.
- **Framework Support:** Adding more framework patterns (SQLAlchemy, GraphQL).
- **Performance:** Optimizing for very large codebases (1M+ lines).

## Development Tooling

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
| **cargo-llvm-cov**      | Code coverage reports           | `cargo llvm-cov`               |
| **cargo-mutants**       | Mutation testing (test quality) | `cargo mutants`                |
| **cargo-semver-checks** | Semver violation detection      | `cargo semver`                 |

### Clippy (Linting)

Clippy is a Rust linting tool that provides additional checks beyond the standard Rust compiler. It is configured via `Cargo.toml` workspace lints and `clippy.toml`. Pedantic lints are enabled.

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

### Code Coverage (cargo-llvm-cov)

Generate coverage reports to see which code paths are tested:

```bash
# Generate HTML coverage report
cargo llvm-cov --html

# Generate LCOV format (for CI)
cargo llvm-cov --lcov --output-path lcov.info

# View summary only
cargo llvm-cov report --summary-only

# With specific features
cargo llvm-cov --all-features
```

> [!NOTE] > `cargo-llvm-cov` is the preferred tool as it works cross-platform. CI uses this for Codecov integration.

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
- [x] **CI Integration**: GitHub Actions workflows added (see `security.yml` and `ci.yml`):
  - `cargo audit` on every PR ✅
  - `cargo deny check` for license compliance ✅
  - `cargo machete` to catch unused dependencies ✅
  - `cargo nextest run` for test execution ✅

---

## Binary Size Optimization

CytoScnPy prioritizes a small binary size for easy distribution. When contributing, please adhere to these optimization strategies:

### 1. "Ruthless" Compiler Settings

We use aggressive optimization in `[profile.release]` (`Cargo.toml`):

- `opt-level = "z"`: Optimize for size.
- `lto = "fat"`: Maximum link-time optimization across all crates.
- `panic = "abort"`: Removes stack unwinding code.
- `codegen-units = 1`: Single compilation unit for better optimization context.
- `strip = true`: Removes debug symbols.

### 2. Linker Flags

Windows builds use strict linker flags in `.cargo/config.toml`:

- `/OPT:REF`: Removes unreferenced functions/data.
- `/OPT:ICF`: Merges identical functions (Identical COMDAT Folding).
- `link-dead-code=no`: Prevents the linker from keeping dead code.

### 3. Dependency Management

- **Trim Features**: Always disable `default-features` for large dependencies (e.g., `clap`, `serde`, `tokio`). Enable only what is strictly needed.
- **No UPX**: We explicitly **do not** use UPX compression because it triggers antivirus false positives and slows down startup. We rely on pure compiler/linker optimizations.

### 4. Profile-Guided Optimization (PGO)

Release builds use PGO for optimal performance. PGO profiles are stored in `pgo-profiles/`.

```bash
# Load PGO profile (Linux/macOS)
source scripts/load-pgo-profile.sh auto

# Load PGO profile (Windows PowerShell)
. scripts/load-pgo-profile.ps1 -Platform windows

# Then build with PGO
cargo build --release
```

---

## Coding Guidelines

- **Formatting:** Always run `cargo fmt` before committing.
- **Linting:** Ensure `cargo clippy` passes without warnings.
- **Error Handling:** Use `anyhow::Result` for application-level errors.
- **Documentation:** Add `///` doc comments for public structs and functions.
- **Tests:** Add unit tests for new logic in the same file or in `tests/`.

## Testing

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
# Quick run with uv (recommended - builds and tests in one command)
uv run --with pytest pytest python/tests

# Or with virtual environment activated
# Install dev dependencies
uv pip install -e ".[dev]"

# Run all Python CLI tests
pytest python/tests/ -v

# Run specific test file
pytest python/tests/test_cli.py -v
pytest python/tests/test_integration.py -v
pytest python/tests/test_json_output.py -v
```

### Rust Edge Case Test Suite

The Rust implementation includes a comprehensive test suite in `cytoscnpy/tests/` with **73 test files** covering 100+ edge cases and real-world scenarios.

#### **Test Suite Overview:**

- **73 test files** covering all analyzer functionality
- **Isolated fixtures** using `tempfile` crate
- **Covers advanced Python patterns** (decorators, async/await, metaclasses, etc.)
- **Tests all CLI flags** (--danger, --quality, --secrets, --fail-threshold)
- **Framework detection tests** for Flask, FastAPI, Django

#### **Running Rust Tests:**

```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test edge_cases_test

# Run specific test
cargo test --test edge_cases_test test_nested_functions

# Run with output
cargo test -- --nocapture

# Run in release mode (faster)
cargo test --release

# Run with logging
RUST_LOG=debug cargo test
```

#### **Key Test Files:**

| Test File             | Coverage                                      |
| --------------------- | --------------------------------------------- |
| `edge_cases_test.rs`  | Comprehensive edge cases (~44 test functions) |
| `framework_test.rs`   | Flask, Django, FastAPI detection              |
| `security_test.rs`    | Secrets and dangerous code detection          |
| `quality_test.rs`     | Code quality checks                           |
| `taint_*_test.rs`     | Taint analysis (8 test files)                 |
| `visitor_test.rs`     | AST visitor unit tests                        |
| `integration_test.rs` | End-to-end binary tests                       |
| `cli_*_test.rs`       | CLI flag and output tests                     |

#### **Test Categories Covered:**

- **Nested Structures**: Deeply nested functions/classes, factory patterns
- **Decorators**: Custom, framework (@route), properties, static/class methods
- **Imports**: Aliasing, circular, relative, conditional, TYPE_CHECKING
- **OOP**: Inheritance, mixins, metaclasses, dataclasses, iterators
- **Advanced Python**: Async/await, generators, walrus operator, match statements
- **Code Quality**: Complexity, nesting, argument count, line count
- **Security**: SQL injection, command injection, pickle, taint analysis
- **Edge Cases**: Empty files, unicode identifiers, long names
- **Clone Detection**: Type-1/2/3 clones, similarity thresholds
- **Control Flow Graph**: CFG construction, behavioral validation

See [`cytoscnpy/tests/README.md`](cytoscnpy/tests/README.md) for detailed test documentation.

## ❓ Getting Help

If you have questions, feel free to open an issue with the `question` label or start a discussion on GitHub.

### Testing-Specific Questions

- **How do I add a new test?** See `cytoscnpy/tests/edge_cases_test.rs` for patterns and fixtures, or `cytoscnpy/tests/README.md` for full documentation.
- **Why are tests skipped?** Some tests require the compiled binary. Run `cargo build` first.
- **Can I test without the Rust binary?** Yes—use `cargo test` to run the Rust test suite directly.
- **How do I validate parity?** Run `cargo test` for Rust tests and `pytest python/tests/` for Python CLI wrapper tests.
