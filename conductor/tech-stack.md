# Tech Stack: CytoScnPy

## Languages
*   **Rust (Core):** High-performance analysis engine and core logic.
*   **Python (Interface):** User-facing CLI, bindings, and integration tests.

## Backend / Core Analysis (Rust)
*   **PyO3:** High-level Rust bindings for the Python interpreter.
*   **Serde:** Framework for serializing and deserializing Rust data structures efficiently.
*   **Workspace Crates:**
    *   `cytoscnpy`: Core analysis library.
    *   `cytoscnpy-cli`: Command-line interface logic.
    *   `cytoscnpy-mcp`: Model Context Protocol server implementation.

## Build and Deployment
*   **Maturin:** Build system for Rust-based Python packages.
*   **Cargo:** Rust's package manager and build tool.
*   **Pip:** Python package installer.

## Tooling and Testing
*   **Typer:** Library for building CLI applications in Python.
*   **Pytest:** Framework for Python unit and integration testing.
*   **Ruff:** Extremely fast Python linter.
*   **Pre-commit:** Framework for managing git pre-commit hooks.

## AI Integration
*   **MCP (Model Context Protocol):** Standard for connecting AI assistants to local data and tools.
