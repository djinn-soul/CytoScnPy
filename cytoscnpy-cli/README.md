# CytoScnPy CLI

Standalone command-line binary for CytoScnPy.

## Purpose

This is a thin wrapper around the `cytoscnpy` library crate that provides a standalone binary entry point.

## Usage

```bash
# Build
cargo build --release --package cytoscnpy-cli

# Run from source
cargo run --package cytoscnpy-cli -- /path/to/project --json
```

## Structure

- `src/main.rs` - Minimal binary entry point
- Depends on `../cytoscnpy` library for all functionality

## Note

For Python users, use the `cytoscnpy` command installed via `pip` or `maturin develop` instead of building this binary directly.

See [../README.md](../README.md) for full usage documentation.
