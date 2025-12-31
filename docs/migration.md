# Migration Guide

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
