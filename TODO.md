# CytoScnPy - Remaining Tasks

> **Focus:** Active & Upcoming Work
> **Sync Status:** Aligned with [ROADMAP.md](ROADMAP.md)

This document tracks the **remaining** work for the CytoScnPy Rust implementation.

---

## 🚧 Active & Upcoming Tasks

### Phase 9: Developer Experience

_Goal: Improve workflow and tooling._

- [ ] **LSP Server**

  - [ ] Implement Language Server Protocol (LSP).
  - [ ] Integrate with `tower-lsp` crate.
  - [ ] Provide diagnostics on `textDocument/didChange`.

- [ ] **Git Integration**

  - [ ] **Blame Analysis:** Identify who introduced unused code.
  - [ ] **Incremental Analysis:** Analyze only changed files (diff with main).

### Phase 10: Deep Analysis & Security

_Goal: Push boundaries of static analysis._

- [ ] **Dependency Graph**

  - [ ] Generate DOT/Mermaid module graphs.

- [ ] **License Compliance**
  - [ ] Scan `requirements.txt` / `Cargo.toml` for license compatibility.

### Phase 11: Auto-Remediation

_Goal: Safe, automated code fixes._

- [ ] **Safe Code Removal (`--fix`)**
  - [ ] **Strategy:** Evaluate `RustPython` ranges vs `tree-sitter`.
  - [ ] **Implementation:** Precise string manipulation to preserve whitespace.

---

## 📦 Release Checklist

- [ ] **Publish to PyPI**
  - [ ] Verify `pyproject.toml` metadata.
  - [ ] Run `maturin publish`.
  - [ ] Verify installation: `pip install cytoscnpy`.
