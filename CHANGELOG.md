## [1.0.0] - 2025-12-08

### 🚀 New Features

- **Taint Analysis:** Track data flow from user inputs (Flask `request` objects) to dangerous sinks (`eval`, `subprocess`, SQL).
  - Detect SQL injection, command injection, and code execution vulnerabilities.
  - Framework-aware source detection for Flask, FastAPI, and Django.
- **Secret Scanning 2.0:** Enhanced regex scanning with Shannon entropy analysis.
  - Reduces false positives for random strings and placeholder values.
  - Better detection of high-entropy API keys and tokens.
- **Type Inference:** Basic type inference for method misuse detection.
  - Detects patterns like `str.append()` (string has no append method).
  - Focus on fast, local, heuristic-based inference.
- **Continuous Benchmarking:** Created benchmark suite with regression detection.
  - Comprehensive ground truth with 126 test items across 6 categories.
  - Comparison against 9 tools (Vulture, Skylos, Ruff, Flake8, Pylint, etc.).
  - CI integration with `coverage.yml` workflow.

### 📚 Documentation

- Updated `BENCHMARK.md` with detailed per-category analysis and tool comparisons.
- Fixed broken links in `README.md` and `CONTRIBUTING.md`.
- Updated `ROADMAP.md` and `TODO.md` to reflect completed benchmarking work.

---

## [0.0.3] - 2025-12-01

### 🚀 New Features

- **Entry Point Detection:** Implemented application entry point with CLI parsing and Python `__name__ == "__main__"` block detection (commit 6ea4806).
  - Detects function calls within `if __name__ == "__main__":` blocks.
  - Prevents false positives for functions called from script entry points.
  - Supports both `__name__ == "__main__"` and `"__main__" == __name__` patterns.
- **Python Linter & Security Analyzer:** Added comprehensive linter with rules, tests, and examples (commit 673ed60).

  - Security rules for detecting dangerous patterns.
  - Quality rules for code complexity checks.
  - Extensive test coverage for all rule sets.

- **Rust Port:** Established hybrid Python/Rust architecture with PyO3 integration (commit b2bf820).
  - Created Rust CLI with `clap` for argument parsing.
  - PyO3 bindings for Python package integration.
  - Parallel file processing with `rayon`.

### 📚 Documentation

- **Roadmap Consolidation:** Consolidated `future.md` and `TODO.md` into new `ROADMAP.md` (commit c5efac8).
  - Clearer project direction and feature planning.
  - Better organization of future enhancements.

### 🐛 Bug Fixes

- **Build System:** Fixed PyO3 Python interpreter detection issues.
  - Added `.cargo/config.toml` with automatic `VIRTUAL_ENV` detection.
  - Eliminates need for manual `PYO3_PYTHON` environment variable setup.
  - Works seamlessly on Windows, Linux, and macOS.
- **Function Signatures:** Fixed missing parameters in command functions (`run_raw`, `run_cc`, `run_hal`, `run_mi`).
- **Ownership Issues:** Fixed Rust borrow checker errors when passing `Config` to `CytoScnPy::new()`.
- **Test Suite:** Updated test function calls to match new signatures in `cli_metrics_test.rs`.

### ✅ Testing

- **Test Coverage:** All 119+ tests pass successfully.
  - 5 library unit tests.
  - 114+ integration tests across 30+ test files.
  - Full coverage for analyzer, CLI, security, visitor, and metrics.

## [0.0.2] - 2025-11-29

### 🚀 New Features

- **Radon Metrics:** Implemented comprehensive code metrics:
  - **Raw Metrics:** LOC, LLOC, SLOC, comments, blanks.
  - **Halstead Metrics:** Difficulty, effort, volume, time.
  - **Cyclomatic Complexity:** Control flow complexity analysis.
- **Dynamic Analysis:**
  - **`hasattr` Support:** Correctly tracks attributes accessed via `hasattr(obj, "attr")`.
  - **Dynamic Patterns:** Enhanced detection for `eval`, `exec`, and `globals` usage.
- **Local Scope Tracking:** Implemented full local scope tracking with `local_var_map` parity.
- **CLI Enrichment:**
  - **Rich Output:** Added beautiful tabular output for results.
  - **Summary Reports:** Detailed summary of issues by category.

## [0.0.1] - 2025-11-28

### 🚀 New Features

- **CLI Enhancements:** Added `--include-folder` flag to explicitly include specific directories in the analysis, overriding default excludes (e.g., useful for analyzing specific `venv` or `build` directories).
- **Advanced Heuristics:** Implemented advanced heuristics to reduce false positives:
  - **Settings/Config Classes:** Uppercase variables in classes ending with `Settings` or `Config` are ignored.
  - **Visitor Pattern:** Methods starting with `visit_`, `leave_`, or `transform_` are marked as used.
  - **Dataclasses:** Fields in `@dataclass` decorated classes are marked as used.
- **Unused Parameter Detection:** Implemented detection of unused function parameters with 90% accuracy.
  - Tracks positional, keyword-only, `*args`, `**kwargs` parameters
  - Never reports `self` or `cls` as unused
  - Applies 70% confidence for interface compliance filtering
  - 11/11 tests passing, 10-14x faster than Python
- **Base Class Tracking:** The analyzer now tracks base classes in class definitions, enabling smarter inheritance-based heuristics.
- **`__all__` Export Detection:** Implemented detection of names exported via `__all__`, preventing them from being flagged as unused.
- **Qualified Name References:** Improved tracking of qualified names (e.g., `module.Class`) to reduce false positives.
- **Core Analyzer:** Implemented the primary AST visitor engine using `rustpython-parser` and `rayon` for parallel file processing.
- **CLI:** Added a command-line interface using `clap` with flags for confidence thresholds and rule selection (`--secrets`, `--danger`, `--quality`).
- **Rule Sets:**
  - **Secrets:** Regex-based scanning for AWS keys, generic API keys, and tokens.
  - **Danger:** Detection of dangerous patterns like `eval`, `exec`, and `subprocess` usage.
  - **Quality:** Code complexity checks, specifically flagging deeply nested code.
- **Pragma Support:** Added support for inline suppression. Lines marked with `# pragma: no cytoscnpy` are now ignored by the analyzer.
- **Entry Point Detection:** Added logic to detect function calls within `if __name__ == "__main__":` blocks to prevent false positives.

### 🚀 Feature Parity & Accuracy

- **Unused Variables:** Implemented unused variable detection, achieving parity with Python (and exceeding it in accuracy).
- **Dynamic Attribute Detection:** Implemented `hasattr` detection to correctly identify dynamically accessed attributes (fixing `Colors.GREEN` false positive).
- **Statement Handling:** Added support for `Assert`, `Raise`, and `Delete` statements to ensure full AST traversal.
- **Variable Tracking:** Improved variable definition tracking, including support for default arguments and type annotations.
- **Variable Tracking:** Improved variable definition tracking, including support for default arguments and type annotations.

### ⚡ Performance

- **Speed:** Optimized analyzer is now **8.7x faster** than the Python implementation (0.78s vs 6.77s).

### 🐛 Bug Fixes

- **Default Excludes:** Fixed an issue where default exclude folders (like `.venv`) were being ignored.
- **Test File Detection:** Fixed the regex for detecting test files to correctly identify and exclude them.
- **Import Handling:** Fixed `ImportFrom` statement handling to correctly track qualified names.
- **Eval Detection:** Expanded `DangerVisitor` to detect `eval()` and `exec()` calls within assignments, return statements, and control flow structures.
- **Method Chaining:** Fixed `CytoScnPyVisitor` to correctly detect method calls in chains (e.g., `obj.method().other()`).
- **Security Tests:** Fixed a test case in `security_test.rs` where a fake Stripe key was too short to match the regex.

### 📚 Documentation

- **Usage Guide:** Updated `README.md` with examples for the new `--include-folder` option.
- **Roadmap:** Updated `future.md` to reflect the completion of this feature.
- Updated `future.md` roadmap to mark parameter detection complete
- Added parameter comparison documentation
- Added parameter comparison documentation
- **Rust Docs:** Created dedicated `README.md` and `CONTRIBUTING.md` for the Rust implementation in `cytoscnpy/`.
- **Roadmap:** Moved `future.md` to `cytoscnpy/` and updated relative links.
- **Benchmarks:** Updated `rust_vs_python_benchmark.md` with the latest performance metrics (~79x faster) and accuracy improvements.
- **Architecture:** Structured the project with a clear library/binary split (`src/lib.rs` and `src/main.rs`).
- **Test Suite:** Established a comprehensive testing infrastructure covering Integration, Unit, and Fixture-based tests.
- **Documentation:** Added implementation docs for Entry Point detection, Pragma support, and Contribution guides.
