# Migration Guide

## Upgrading to v1.2.5

### CI Workflow Improvements

- **Python 3.13 Support**: If you are using `pip install` on Linux with Python 3.13, you no longer need the standalone installer as a workaround. Wheels are now explicitly built for Python 3.9 through 3.13 for all supported platforms.

## Upgrading to v1.2.2

### Behavioral Changes

- **Test Exclusion**: Starting from v1.2.2, tests are **excluded by default** in both the CLI and library API to reduce noise. Use the `--include-tests` flag if you want to scan your test files.

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
