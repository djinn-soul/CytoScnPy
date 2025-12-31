# CytoScnPy Rust Core

This directory contains the Rust implementation of CytoScnPy, a high-performance Python static analyzer. This crate is the core of the cytoscnpy ecosystem, providing both a Rust library and a Python extension module.

## Package Structure

This is a hybrid Rust crate that serves two purposes:

1.  **Library Crate (`cytoscnpy`)**: It is compiled as a Rust library (`rlib`) for use in other Rust crates (like `cytoscnpy-cli`) and as a dynamic library (`cdylib`) to create Python bindings with PyO3.
2.  **Binary Crate (`cytoscnpy-bin`)**: It also contains a binary target, `cytoscnpy-bin`, which is a command-line interface for the analyzer.

The primary, user-facing CLI executable is provided by the `cytoscnpy-cli` crate in the parent directory, which is a thin wrapper around this library.

### Key Files

- `src/lib.rs` - Library root, PyO3 module definition, and core logic.
- `src/main.rs` - The entry point for the `cytoscnpy-bin` binary.
- `src/python_bindings.rs` - PyO3 function implementations for the Python extension.
- `src/analyzer/` - Main analysis orchestration logic and dead code detection.
- `src/visitor.rs` - The core AST traversal and analysis logic.
- `src/rules/` - Directory containing modules for specific checks (secrets, danger, quality).
- `src/taint/` - Taint analysis engine (intraprocedural, interprocedural, cross-file).
- `src/complexity.rs` - Cyclomatic complexity calculation.
- `src/halstead.rs` - Halstead metrics calculation.
- `src/raw_metrics.rs` - Raw code metrics (LOC, SLOC, etc.).
- `src/config.rs` - Logic for handling configuration from `pyproject.toml` or `.cytoscnpy.toml`.

## 🔒 Security Analysis

CytoScnPy includes a powerful security engine written in Rust.

### Taint Analysis (v1.0.0)

Tracks data flow from untrusted sources to dangerous sinks:

- **Intraprocedural**: Checks flows within single functions.
- **Interprocedural**: Checks flows across functions in the same file.
- **Cross-file**: Checks flows across module boundaries.
- **Detections**: SQL injection, command injection, code execution, path traversal.

### Secret Scanning

- Uses regex patterns for AWS keys, API tokens, private keys.
- **Shannon Entropy Analysis**: Reduces false positives by analyzing the randomness of the string.
- Detects high-entropy strings that look like real secrets but don't match known prefixes.

### Dangerous Code Patterns

- `eval()`, `exec()`, `compile()` detection.
- `pickle` deserialization warnings.
- `subprocess` shell injection risks.

## 🏗️ Architecture

```
CytoScnPy/
├── cytoscnpy/                    # Rust core library
│   └── src/
│       ├── analyzer/             # Core analysis engine
│       │   ├── mod.rs            # Module exports
│       │   ├── types.rs          # AnalysisResult, ParseError
│       │   ├── heuristics.rs     # Penalty and heuristic logic
│       │   └── processing.rs     # Core processing methods
│       ├── visitor.rs            # AST visitor implementation
│       ├── rules/                # Security & quality rules
│       │   ├── mod.rs            # Rules module
│       │   ├── danger.rs         # Dangerous code detection
│       │   ├── secrets.rs        # Secret scanning + entropy
│       │   └── quality.rs        # Code quality checks
│       ├── taint/                # Taint analysis engine
│       │   ├── mod.rs            # Module exports
│       │   ├── types.rs          # TaintFinding, VulnType
│       │   ├── analyzer.rs       # Main taint analyzer
│       │   ├── sources.rs        # User input sources
│       │   ├── sinks.rs          # Dangerous sinks
│       │   ├── propagation.rs    # Taint state tracking
│       │   ├── intraprocedural.rs
│       │   ├── interprocedural.rs
│       │   ├── crossfile.rs      # Cross-module analysis
│       │   ├── call_graph.rs     # Function call graph
│       │   └── summaries.rs      # Function summaries
│       ├── clones/               # Clone detection
│       │   ├── mod.rs            # CloneDetector orchestrator
│       │   ├── config.rs         # CloneConfig settings
│       │   ├── parser.rs         # Subtree extraction
│       │   ├── similarity.rs     # Tree similarity & edit distance
│       │   ├── hasher.rs         # LSH candidate pruning
│       │   └── confidence.rs     # Fix confidence scoring
│       ├── cfg/                  # Control Flow Graph (feature: cfg)
│       │   └── mod.rs            # CFG construction & fingerprinting
│       ├── complexity.rs         # Cyclomatic complexity
│       ├── halstead.rs           # Halstead metrics
│       ├── raw_metrics.rs        # LOC/SLOC counting
│       └── python_bindings.rs    # PyO3 integration
│
├── cytoscnpy-cli/                # Standalone CLI binary
├── python/                       # Python package wrapper
└── benchmark/                    # 135-item ground truth suite
```

### Technology Stack

| Component           | Technology                                         |
| ------------------- | -------------------------------------------------- |
| **Parser**          | `ruff_python_parser` (Python 3.12+ compatible)     |
| **Parallelization** | `rayon` for multi-threaded file processing         |
| **CLI**             | `clap` with derive macros                          |
| **Python Bindings** | `PyO3` + `maturin` build system                    |
| **Output**          | `colored` + `comfy-table` for rich terminal output |

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

- The Rust library in `target/release/libcytoscnpy.rlib`.
- The binary executable at `target/release/cytoscnpy-bin`.

### Feature Flags

The crate supports optional features that can be enabled at compile time:

| Feature | Description                                                                                 |
| ------- | ------------------------------------------------------------------------------------------- |
| `cfg`   | Enables Control Flow Graph (CFG) construction and behavioral validation for clone detection |

To build with a feature enabled:

```bash
# Build with CFG support
cargo build --features cfg

# Build release with CFG support
cargo build --release --features cfg
```

## Testing

Run the tests for this specific crate using Cargo.

```bash
# Run all tests for the cytoscnpy crate
cargo test

# Run tests with CFG feature enabled
cargo test --features cfg
```
