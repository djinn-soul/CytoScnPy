# CytoScnPy Rust Core

This directory contains the Rust implementation of CytoScnPy, a high-performance Python static analyzer. This crate is the core of the cytoscnpy ecosystem, providing both a Rust library and a Python extension module.

## Package Structure

This is a hybrid Rust crate that serves two purposes:

1.  **Library Crate (`cytoscnpy`)**: It is compiled as a Rust library (`rlib`) for use in other Rust crates (like `cytoscnpy-cli`) and as a dynamic library (`cdylib`) to create Python bindings with PyO3.
2.  **Binary Crate (`cytoscnpy-bin`)**: It also contains a binary target, `cytoscnpy-bin`, which is a command-line interface for the analyzer.

The primary, user-facing CLI executable is provided by the `cytoscnpy-cli` crate in the parent directory, which is a thin wrapper around this library.

### Key Files

-   `src/lib.rs` - Library root, PyO3 module definition, and core logic.
-   `src/main.rs` - The entry point for the `cytoscnpy-bin` binary.
-   `src/python_bindings.rs` - PyO3 function implementations for the Python extension.
-   `src/analyzer.rs` - Main analysis orchestration logic.
-   `src/visitor.rs` - The core AST traversal and analysis logic.
-   `src/rules/` - Directory containing modules for specific checks (e.g., security, quality).
-   `src/config.rs` - Logic for handling configuration from `pyproject.toml` or `.cytoscnpy.toml`.

## Building

This library is a dependency of the main `cytoscnpy` Python package and the `cytoscnpy-cli` tool.

### Building the Python Wheel

To build the Python extension, you can use `maturin`. Run this command from the workspace root (`E:\Github\CytoScnPy`):

```bash
# Ensure you are in the root of the repository
maturin develop -m cytoscnpy/Cargo.toml
```

### Building the Rust Library and Binary

To build the Rust components directly, you can use Cargo.

```bash
# From this directory (E:\Github\CytoScnPy\cytoscnpy)
cargo build --release
```

This will produce:
-   The Rust library in `target/release/libcytoscnpy.rlib`.
-   The binary executable at `target/release/cytoscnpy-bin`.

## Testing

Run the tests for this specific crate using Cargo.

```bash
# Run all tests for the cytoscnpy crate
cargo test
```
