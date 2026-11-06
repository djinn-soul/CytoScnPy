# Migration Guide

## Upgrading to v1.3.0

### Compatibility

- **Python 3.14 Support**: Wheels are built and tested on Linux, Windows, and macOS. The minimum supported version remains Python 3.10.
- **Windows**: The binaries statically link the C runtime, so `VCRUNTIME140.dll` no longer needs to be installed.
- **Pre-commit Hooks**: Update the hook `rev` to `v1.3.0`.

### Breaking Changes

- **JSON schema**: The CLI JSON output field `clone_findings` has been renamed to `clones`. Update consumers to read `clones` instead.

### Behavioral Changes

- **Rule-aware suppressions**: `# noqa: <codes>` now hides only the findings whose rule ID is listed. A comment written for another tool, such as `# noqa: E501`, no longer hides CytoScnPy findings, including clone findings. Use bare `# noqa`, `# noqa: CSP`, or the finding's own rule ID instead.
- **Dead code suppression**: Dead code findings honour `CSP-V001` and per-kind IDs such as `unused-import` and `unused-function`.
- **Suppression parsing**: A `#` inside a string or docstring is no longer read as a comment. Text after the codes is treated as an explanation (`# noqa: CSP-D003 input is validated`), and `ignore` must be a whole word (`# ignored` does not suppress).
- **Deslop on an empty scan**: `cytoscnpy deslop` on a path with no Python files now fails the `no_python_files` gate and exits with code `1` instead of reporting a clean score.
- **Deslop output format**: An explicit `--format` decides the output even when `--json` is also passed.

## Upgrading to v1.2.5

### CI Workflow Improvements

- **Python 3.13 Support**: If you are using `pip install` on Linux with Python 3.13, you no longer need the standalone installer as a workaround. Wheels are now explicitly built for Python 3.9 through 3.13 for all supported platforms.

## Upgrading to v1.2.2

### Behavioral Changes

- **Test Exclusion**: Starting from v1.2.2, tests are **excluded by default** in the CLI and library API to reduce noise. This applies consistently to main analysis, standalone metrics, file statistics, and clone detection. Use `--include-tests` to scan test files. Dependency analysis remains test-aware by design and scans test/dev imports separately from production imports.

## Upgrading to v1.2.0

### Breaking Changes

- **Pre-commit Hooks**: The hook `rev` should be updated to `v1.2.1` in your `.pre-commit-config.yaml`.
- **Parser Engine**: Switched from `rustpython-parser` to `ruff_python_parser`. This improves performance and Python 3.12+ compatibility but may handle syntax errors more strictly.

### Command Line Interface

- **Subcommands**: Metric calculations are now grouped under subcommands (`raw`, `cc`, `hal`, `mi`, `stats`, `files`).
  - Old: `cytoscnpy --raw` (hypothetical legacy)
  - New: `cytoscnpy raw .`
- **Output**: The `stats` subcommand with `--all` is the recommended default for CI pipelines requiring full analysis.

### Configuration

- **Notebooks**: `include_ipynb` and `ipynb_cells` are currently **CLI-only** flags. Verify they are passed in your command line arguments, as they are not yet supported in `.cytoscnpy.toml`.

## Upgrading from v1.0.x

### Feature Flags

- **CFG Support**: Control Flow Graph analysis is now an opt-in feature. Build with `--features cfg` if you rely on deep behavioral analysis for clone detection.

### Python API

- **Entry Point**: The `cytoscnpy.run()` function is the stable entry point. Direct access to internal modules is not guaranteed.
