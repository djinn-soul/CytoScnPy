## [1.0.1] - 2025-12-10
- **Unused Method Detection:** Added support for detecting unused methods.
- **Fail Threshold:** Added `fail_threshold` config to exit with code 1 if unused code exceeds limit.
- **Metaclass Support:** Improved detection for classes inheriting from metaclass registries.
- **CLI:** Better error handling for invalid file paths.
- **JSON Output:** Added method type for backward compatibility.
- **Documentation:** Fixed broken links in README and alerts in CONTRIBUTING.
- **Diagrams:** Updated project structure and architecture diagrams.
- **Changelog:** Cleaned up duplicate entries.
- **Testing:** Added tests for metaclass usage and inheritance.
- **Testing:** Added `metaclass_patterns.py` benchmark.

## [1.0.0] - 2025-12-08

- **Taint Analysis:** Tracks data flow from inputs (Flask, FastAPI, Django) to sinks (SQL, eval) to detect injection vulnerabilities.
- **Secret Scanning 2.0:** Enhanced regex scanning with Shannon entropy analysis to reduce false positives.
- **Type Inference:** Heuristic-based inference for method misuse detection (e.g., `str.append()`).
- **Continuous Benchmarking:** Regression detection suite with 126 test items comparing against 9 tools (Vulture, Ruff, etc.).
- **Entry Point Detection:** Supports CLI parsing and `if __name__ == "__main__":` blocks to prevent false positives.
- **Python Linter & Security Analyzer:** Comprehensive rules for security patterns and code complexity.
- **Rust Port:** Hybrid Python/Rust architecture using PyO3, `clap` for CLI, and `rayon` for parallel processing.
- **Radon Metrics:** Calculates Raw (LOC), Halstead, and Cyclomatic Complexity metrics.
- **Dynamic Analysis:** Improved tracking for `hasattr`, `eval`, `exec`, and `globals`.
- **Local Scope Tracking:** Full local scope tracking with `local_var_map` parity.
- **CLI Enrichment:** Rich tabular output and detailed summary reports.
- **CLI Enhancements:** Added `--include-folder` flag to override default excludes.
- **Advanced Heuristics:** Reduces false positives for Settings/Config classes, Visitor patterns, and Dataclasses.
- **Unused Parameter Detection:** Detects unused function parameters with high accuracy.
