# CytoScnPy - Development History

> This document contains the history of completed features and milestones.
> For upcoming features, see [ROADMAP.md](ROADMAP.md).

---

## Table of Contents

1. [Phase 1: Critical Fixes](#phase-1-critical-fixes)
2. [Phase 2: Feature Parity](#phase-2-feature-parity)
3. [Phase 3: Advanced Features](#phase-3-advanced-features)
4. [Phase 4: Code Architecture](#phase-4-code-architecture)
5. [Phase 5: Radon Metrics Integration](#phase-5-radon-metrics-integration)
6. [Phase 6: Editor Integration](#phase-6-editor-integration)
7. [Phase 7: Infrastructure & Quality](#phase-7-infrastructure--quality)
8. [Phase 8: Framework Support](#phase-8-framework-support)

---

## Parser Migration: rustpython-parser → ruff_python_parser ✅

Completed full migration from the deprecated `rustpython-parser` to `ruff_python_parser`:

**Key Changes:**

- Updated `Cargo.toml` with ruff git dependencies
- Merged async variants (`AsyncFunctionDef` → `FunctionDef.is_async`)
- Updated parameter access (`args` → `parameters`)
- Fixed f-string handling with new `InterpolatedStringElement` structure
- All 200+ tests pass

**Benefits:**

- Improved Python 3.12+ match statement support
- Better performance from optimized Ruff parser
- Avoids deprecated `rustpython-parser` with known `unic` vulnerability

---

## Phase 1: Critical Fixes ✅

Foundational fixes addressing the largest sources of false positives.

### 1.1 Import Resolution (Aliasing) ✅

**Problem:** Rust didn't track import aliases, causing false positives:

```python
import pandas as pd
df = pd.DataFrame()  # Rust reported 'pandas' as unused
```

**Solution:** Added `alias_map: HashMap<String, String>` to `CytoScnPyVisitor`.

**Impact:** Eliminated ~79 false positives

---

### 1.2 Method and Class Context ✅

**Problem:** Methods weren't qualified with class names:

```python
class MyClass:
    def method(self): pass
    def caller(self):
        self.method()  # Rust didn't recognize this
```

**Solution:** Added `class_stack: Vec<String>` to track class nesting.

**Impact:** Eliminated ~184 false positives

---

### 1.3 Qualified Name Matching ✅

Implemented smart name resolution checking multiple qualification levels:

1. Exact match
2. Module.name match
3. Class.method match
4. Partial qualified match

**Impact:** Improved accuracy by ~15%

---

## Phase 2: Feature Parity ✅

Bringing Rust implementation to feature parity with Python version.

### 2.1 Pragma Support ✅

```python
def unused_function():  # pragma: no cytoscnpy
    pass  # Won't be reported as unused
```

Created `get_ignored_lines()` in `src/utils.rs`.

---

### 2.2 Configuration File Support ✅

- `.cytoscnpy.toml` in project root
- `pyproject.toml` under `[tool.cytoscnpy]`
- CLI arguments take precedence

---

### 2.3 Unused Parameter Detection ✅

```python
def process(data, unused_param):  # 'unused_param' flagged
    return data
```

Added `function_params` map with 70% confidence (vs 100% for other code).

---

### 2.4 Advanced Heuristics ✅

1. **Settings/Config Classes:** Uppercase variables in `*Settings`/`*Config` classes ignored
2. **Visitor Pattern:** Methods starting with `visit_`, `leave_`, `transform_` auto-referenced
3. **Dataclass Fields:** Fields in `@dataclass` decorated classes marked as used
4. **Dunder Methods:** `__init__`, `__str__`, etc. get lower confidence penalty

---

### 2.5 `__all__` Export Detection ✅

Parses `__all__ = [...]` and marks corresponding definitions as exported.

---

### 2.6 Rich CLI Output ✅

- Progress Spinner: `indicatif`
- Tabular Results: `comfy-table` with box-drawing characters
- Severity Coloring: Red (Critical), Yellow (Medium), Blue (Info)

---

## Phase 3: Advanced Features ✅

Features exceeding the original Python implementation.

### 3.1 Local Scope Tracking ✅

Added `local_var_map: HashMap<String, String>` to `Scope` struct for accurate variable tracking in complex scopes.

---

### 3.2 Dynamic Code Patterns ✅

1. **Globals Tracking:** `globals()["var"]` marks module as dynamic
2. **Eval/Exec Detection:** Lowers confidence for all definitions
3. **Hasattr Pattern:** `hasattr(obj, "attr")` adds reference to attribute

---

## Phase 4: Code Architecture ✅

### 4.0 Modular Rule System ✅

Refactored into trait-based architecture:

```rust
pub trait Rule {
    fn name(&self) -> &str;
    fn enter_stmt(&mut self, stmt: &Stmt) -> Option<Finding>;
    fn leave_stmt(&mut self, stmt: &Stmt) -> Option<Finding>;
    fn visit_expr(&mut self, expr: &Expr) -> Option<Finding>;
}
```

**Implemented Rules:** Danger, Quality, Secrets, Complexity, Nesting, Arguments.

---

### 4.1 Hybrid PyO3 Distribution ✅

- `cytoscnpy/` - Rust library + `#[pymodule]`
- `python/cytoscnpy/` - Python wrapper
- `cytoscnpy-cli/` - Standalone binary

---

## Phase 5: Radon Metrics Integration ✅

### 5.1 Raw Metrics (LOC/LLOC/SLOC) ✅

Radon-compatible line counting in `src/raw_metrics.rs`.

### 5.2 Halstead Metrics ✅

Vocabulary, Volume, Difficulty, Effort, Bugs in `src/halstead.rs`.

### 5.3 Maintainability Index (MI) ✅

Visual Studio-style 0-100 score with A/B/C ranking.

### 5.4 Cyclomatic Complexity ✅

McCabe complexity with A-F ranking (A: 1-5, F: 41+).

### 5.5 CLI Integration ✅

Commands: `cytoscnpy cc`, `raw`, `hal`, `mi` with `--average`, `--json`, `--xml` flags.

### 5.6 Quality Gates ✅

- `cc --fail-threshold <N>`: Fail if complexity > N
- `mi --fail-under <N>`: Fail if MI < N

---

## Phase 6: Editor Integration ✅

### 6.1 VS Code Extension ✅

Full-featured extension in `editors/vscode/cytoscnpy/`:

| Feature                       | Status |
| ----------------------------- | ------ |
| Real-time diagnostics         | ✅     |
| Configuration parity with CLI | ✅     |
| Status Bar (issue count)      | ✅     |
| Quick Fixes (Remove/Comment)  | ✅     |
| Gutter Decorations            | ✅     |
| File Caching                  | ✅     |
| Problem Grouping              | ✅     |

---

## Phase 7: Infrastructure & Quality ✅

### 7.2 Error Handling ✅

Implemented `ParseError` struct with error table display.

### 7.3 Parser Migration ✅

Migrated from `rustpython-parser` to `ruff_python_parser`.

### 7.5 Performance Optimizations ✅

| Optimization         | Impact          |
| -------------------- | --------------- |
| LTO + FxHashMap      | 22.6% faster    |
| Reference counting   | 41.4% faster    |
| LineIndex + OnceLock | **54.9% total** |

Additional: SmallVec, Arc<PathBuf>, cached scope prefix, chunk-based Rayon.

---

## Phase 8: Framework Support ✅

### Django Support ✅

- URL patterns parsing (`urlpatterns`)
- Admin registration (`admin.site.register`)
- Signal detection (`pre_save.connect`)

### FastAPI Support ✅

- `Depends()` extraction from route handlers

### Pydantic Support ✅

- BaseModel field tracking

---

## Release Milestones

| Milestone               | Status                     |
| ----------------------- | -------------------------- |
| Publish to PyPI         | ✅ `pip install cytoscnpy` |
| MCP Server (Stdio)      | ✅ `cytoscnpy-mcp`         |
| GitHub Actions Workflow | ✅ Cross-platform builds   |
