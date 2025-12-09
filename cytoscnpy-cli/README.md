# CytoScnPy CLI

Standalone command-line binary for CytoScnPy.

## Purpose

This is a thin wrapper around the `cytoscnpy` library crate that provides a standalone binary entry point.

## Usage

```bash
# Build
cargo build --release --package cytoscnpy-cli

# Run dead code analysis
cargo run --package cytoscnpy-cli -- /path/to/project

# Run with all security checks
cargo run --package cytoscnpy-cli -- /path/to/project --secrets --danger --taint

# JSON output for CI/CD
cargo run --package cytoscnpy-cli -- /path/to/project --json

# Metric subcommands
cargo run --package cytoscnpy-cli -- cc /path/to/project  # Cyclomatic complexity
cargo run --package cytoscnpy-cli -- mi /path/to/project  # Maintainability index
```

## Structure

- `src/main.rs` - Minimal binary entry point
- Depends on `../cytoscnpy` library for all functionality

## Note

For Python users, use the `cytoscnpy` command installed via `pip` or `maturin develop` instead of building this binary directly.

See [../README.md](../README.md) for full usage documentation.
