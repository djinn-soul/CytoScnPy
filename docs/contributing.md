# Contributing to CytoScnPy

Thank you for your interest in contributing to CytoScnPy!

## 🛠️ Prerequisites

- **Rust**: Version 1.70 or higher.
- **Cargo**: Comes with Rust.
- **Python**: Version 3.8 or higher.
- **Maturin**: `pip install maturin`

## 🚀 Setup Development Environment

1. **Fork and Clone:**

   ```bash
   git clone https://github.com/YOUR_USERNAME/cytoscnpy.git
   cd cytoscnpy
   ```

2. **Create Virtual Environment:**

   ```bash
   python -m venv .venv
   source .venv/bin/activate  # Linux/macOS
   .venv\Scripts\activate     # Windows
   ```

3. **Install Dependencies & Build:**

   ```bash
   pip install maturin
   maturin develop -m cytoscnpy/Cargo.toml
   ```

4. **Run Tests:**

   ```bash
   cargo test
   pytest python/tests
   ```

## 🔄 Development Workflow

1. **Create a Branch:**
   `git checkout -b feature/your-feature`

2. **Make Changes:**

   - Run `cargo fmt` to format.
   - Run `cargo clippy` to lint.

3. **Test:**

   - `cargo test` (Rust unit tests)
   - `pytest` (Python integration tests)

4. **Submit PR:**
   - Push to your fork.
   - Open a Pull Request on GitHub.

## 📂 Project Structure

- `cytoscnpy/` - Rust core library & analysis engine.
- `python/` - Python wrapper & CLI entry point.
- `editors/vscode/` - VS Code extension.
- `cytoscnpy-mcp/` - MCP server documentation.

## 🧪 Testing

We have a comprehensive test suite.

```bash
# Run all Rust tests
cargo test

# Run specific test
cargo test test_name
```

See [tests/README.md](https://github.com/djinn09/CytoScnPy/tree/main/cytoscnpy/tests) for detailed testing guide.
