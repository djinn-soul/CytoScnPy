# CytoScnPy - Complete Roadmap & Development Guide

> **Architecture:** Hybrid PyO3 + Standalone CLI
> **Status:** Production-ready core, active development

This comprehensive document details the complete development roadmap for CytoScnPy - a high-performance Python static analyzer written in Rust with Python integration.

**Performance Highlights:**

- **Hybrid distribution** - pip installable with both Python API and CLI

---

## 📋 Table of Contents

1. [Project Status](#project-status)
2. [Phase 1: Critical Fixes](#phase-1-critical-fixes-done)
3. [Phase 2: Feature Parity](#phase-2-feature-parity-done)
4. [Phase 3: Advanced Features](#phase-3-advanced-features-done)
5. [Phase 4: Code Architecture](#phase-4-code-architecture-done)
6. [Phase 5: Radon Metrics Integration](#phase-5-radon-metrics-integration-done)
7. [Phase 6: Editor Integration](#phase-6-editor-integration-done)
8. [Phase 7: Infrastructure & Quality](#phase-7-infrastructure--quality-done)
9. [Future Roadmap](#future-roadmap)
   - [Phase 8: Advanced Framework Support](#phase-8-advanced-framework-support)
   - [Phase 9: Developer Experience](#phase-9-developer-experience)
   - [Phase 10: Deep Analysis & Security](#phase-10-deep-analysis--security)
   - [Phase 11: Auto-Remediation](#phase-11-auto-remediation)

---

## Project Status

### Current Capabilities ✅

**Core Detection:**

- Unused functions, classes, methods
- Unused imports (with aliasing support)
- Unused variables and parameters
- Cross-module reference tracking
- Entry point detection (`if __name__ == "__main__"`)

**Security Analysis:**

- Hardcoded secrets (API keys, tokens, private keys)
- SQL injection risks
- Command injection patterns
- Dangerous code (`eval`, `exec`, `pickle`, `yaml.unsafe_load`)
- Weak cryptography (`md5`, `sha1`)

**Code Quality:**

- Cyclomatic complexity (with A-F ranking)
- Halstead metrics (vocabulary, volume, difficulty, effort)
- Raw metrics (LOC, LLOC, SLOC, comments)
- Maintainability Index (0-100 score)
- Nesting depth analysis
- Argument count detection

**Configuration & UX:**

- `.cytoscnpy.toml` and `pyproject.toml` support
- Inline pragma support (`# pragma: no cytoscnpy`)
- Rich CLI output with tables and colors
- JSON output for CI/CD integration
- Progress spinner and file statistics
- Taint analysis (`--taint` flag)

---

## Phase 1: Critical Fixes ✅ DONE

These foundational fixes were essential for accuracy and addressed the largest sources of false positives.

### 1.1 Import Resolution (Aliasing) ✅

**Problem:** Rust didn't track import aliases, causing false positives:

```python
import pandas as pd
df = pd.DataFrame()  # Rust reported 'pandas' as unused
```

**Solution:**

- Added `alias_map: HashMap<String, String>` to `CytoScnPyVisitor`
- Tracks `Import` and `ImportFrom` aliases during AST traversal
- Resolves aliases during reference collection

**Implementation Details:**

```rust
// In visit_import()
if let Some(alias) = &alias.asname {
    self.alias_map.insert(alias.id.to_string(), name.id.to_string());
}
// In visit_name()
let resolved_name = self.alias_map.get(id).unwrap_or(id);
```

**Impact:** Eliminated ~79 false positives in test suite
**Files Modified:** `src/visitor.rs`
**Test Coverage:** `import_resolution_test.rs`

---

### 1.2 Method and Class Context ✅

**Problem:** Methods weren't qualified with class names:

```python
class MyClass:
    def method(self): pass
    def caller(self):
        self.method()  # Rust didn't recognize this as using 'method'
```

**Solution:**

- Added `class_stack: Vec<String>` to track class nesting
- Qualify method definitions: `MyClass.method`
- Qualify `self.method()` calls with current class context
- Added decorator visitation to handle `@property`, etc.

**Implementation Details:**

```rust
// Entering class
self.class_stack.push(class_name.clone());

// Method definition
let qualified_name = if let Some(class_name) = self.class_stack.last() {
    format!("{}.{}", class_name, func_name)
} else {
    func_name.clone()
};
```

**Impact:** Eliminated ~184 false positives
**Files Modified:** `src/visitor.rs`
**Test Coverage:** `method_context_test.rs`, `class_context_test.rs`

---

### 1.3 Qualified Name Matching ✅

**Problem:** References didn't always match definitions due to different qualification levels.

**Solution:**

- Implemented smart name resolution in `resolve_name()`
- Checks multiple qualification levels:
  1. Exact match
  2. Module.name match
  3. Class.method match
  4. Partial qualified match

**Impact:** Improved accuracy by ~15%
**Files Modified:** `src/visitor.rs`

---

## Phase 2: Feature Parity ✅ DONE

Bringing Rust implementation to feature parity with the Python version.

### 2.1 Pragma Support ✅

**Feature:** Inline suppression of warnings

```python
def unused_function():  # pragma: no cytoscnpy
    pass  # Won't be reported as unused
```

**Implementation:**

- Created `get_ignored_lines()` in `src/utils.rs`
- Scans file for `# pragma: no cytoscnpy` comments
- Returns `HashSet<usize>` of ignored line numbers
- Integrated into penalty system (confidence = 0 for ignored lines)

**Files:** `src/utils.rs`, `src/analyzer.rs`
**Test Coverage:** `pragma_test.rs`

---

### 2.2 Configuration File Support ✅

**Feature:** Project-level configuration via `.cytoscnpy.toml` or `pyproject.toml`.

**Implementation:**

- Created `src/config.rs` with `CytoScnPyConfig` struct
- Searches for `.cytoscnpy.toml` in project root
- Falls back to `pyproject.toml` under `[tool.cytoscnpy]`
- Merges config with CLI arguments (CLI takes precedence)

**Priority Order:**

1. CLI arguments (highest priority)
2. `.cytoscnpy.toml`
3. `pyproject.toml` (`[tool.cytoscnpy]`)
4. Defaults

**Files:** `src/config.rs`
**Test Coverage:** `config_test.rs`

---

### 2.3 Unused Parameter Detection ✅

**Feature:** Detect function parameters that are never used.

```python
def process(data, unused_param):  # 'unused_param' flagged
    return data
```

**Implementation:**

- Added `function_stack` and `function_params` map
- Extracts all parameter types (positional, keyword, \*args, \*\*kwargs)
- Automatically skips `self` and `cls`
- Applied **70% confidence** (vs 100% for other code)

**Files:** `src/visitor.rs`
**Test Coverage:** `parameter_test.rs`

---

### 2.4 Advanced Heuristics ✅

Multiple heuristics to reduce false positives for common patterns.

1.  **Settings/Config Classes:**

    - **Pattern:** Uppercase variables in classes named `*Settings` or `*Config`.
    - **Action:** Set confidence = 0 (effectively ignored).

2.  **Visitor Pattern:**

    - **Pattern:** Methods starting with `visit_`, `leave_`, `transform_`.
    - **Action:** Increment reference count (always used).

3.  **Dataclass Fields:**

    - **Pattern:** Fields in `@dataclass` decorated classes.
    - **Action:** Mark all class-level annotations as used.

4.  **Dunder Methods:**
    - **Pattern:** `__init__`, `__str__`, etc.
    - **Action:** Lower confidence penalty.

**Files:** `src/visitor.rs`, `src/analyzer.rs`
**Test Coverage:** `heuristics_test.rs`

---

### 2.5 `__all__` Export Detection ✅

**Feature:** Respect module exports defined in `__all__`.

**Implementation:**

- Parse `__all__ = [...]` assignments in AST
- Extract string literals from the list
- Mark corresponding definitions as exported (used)

**Files:** `src/visitor.rs`

---

### 2.6 Rich CLI Output ✅

**Feature:** Professional, colored, tabular output matching Python version.

**Features:**

- **Progress Spinner:** `indicatif`
- **Tabular Results:** `comfy-table` with box-drawing characters
- **Severity Coloring:** Red (Critical), Yellow (Medium), Blue (Info)
- **Organized Sections:** Summary, Unused Code, Security, Quality

**Files:** `src/output.rs`, `src/main.rs`

---

## Phase 3: Advanced Features ✅ DONE

Features exceeding the original Python implementation.

### 3.1 Local Scope Tracking ✅

**Problem:** Local variables not properly qualified across nested scopes.

**Solution:**

- Added `local_var_map: HashMap<String, String>` to `Scope` struct.
- Maps unqualified name → fully qualified name per scope.
- Enhanced `resolve_name()` to check `local_var_map` first.

**Implementation:**

```rust
pub struct Scope {
    pub name: String,
    pub scope_type: ScopeType,
    pub local_var_map: HashMap<String, String>,  // NEW
}
```

**Impact:** Accurate variable tracking in complex scopes.
**Files:** `src/visitor.rs`
**Test Coverage:** `local_scope_test.rs`

---

### 3.2 Dynamic Code Patterns ✅

1.  **Globals Tracking:**

    - **Pattern:** `globals()["var"]`
    - **Action:** Mark calling module as having dynamic references.

2.  **Eval/Exec Detection:**

    - **Pattern:** `eval(code)`, `exec(code)`
    - **Action:** Mark module as dynamic (lower confidence for all definitions).

3.  **Hasattr Pattern:**
    - **Pattern:** `hasattr(obj, "attr")`
    - **Action:** Add reference to the attribute name.

**Files:** `src/visitor.rs`
**Test Coverage:** `dynamic_patterns_test.rs`

---

## Phase 4: Code Architecture ✅ DONE

### 4.0 Modular Rule System ✅

**Problem:** Monolithic visitors were hard to extend and test.

**Solution:** Refactored into a trait-based architecture.

```rust
pub trait Rule {
    fn name(&self) -> &str;
    fn enter_stmt(&mut self, stmt: &Stmt) -> Option<Finding>;
    fn leave_stmt(&mut self, stmt: &Stmt) -> Option<Finding>;
    fn visit_expr(&mut self, expr: &Expr) -> Option<Finding>;
}
```

**Implemented Rules:**

- **Danger:** Eval/Exec, Pickle, Yaml, Hashlib, Requests, Subprocess, SQL Injection, Command Injection.
- **Quality:** Complexity, Nesting, Argument Count.

**Files:** `src/rules/mod.rs`, `src/linter.rs`

---

### 4.1 Hybrid PyO3 Distribution ✅

**Architecture:** Python package with Rust extension + Standalone CLI.

**Components:**

1.  **`cytoscnpy/` (Rust Library):** Core logic + `#[pymodule]`.
2.  **`python/cytoscnpy/` (Python Wrapper):** CLI wrapper calling Rust.
3.  **`cytoscnpy-cli/` (Standalone Binary):** Minimal binary wrapper.

**Benefits:** Single codebase, multiple interfaces (Python API, CLI, Standalone).

---

## Phase 5: Radon Metrics Integration ✅ DONE

Integration of code metrics compatible with `radon`.

### 5.1 Raw Metrics (LOC/LLOC/SLOC) ✅

**Feature:** Radon-compatible line counting.

- **LOC:** Total lines
- **LLOC:** Logical lines (statements)
- **SLOC:** Source lines (code only)

**Files:** `src/raw_metrics.rs`

### 5.2 Halstead Metrics ✅

**Feature:** Program vocabulary and complexity.

- **Metrics:** Vocabulary, Volume, Difficulty, Effort, Bugs.
- **Implementation:** AST visitor to count operators and operands.

**Files:** `src/halstead.rs`

### 5.3 Maintainability Index (MI) ✅

**Feature:** Visual Studio-style Maintainability Index (0-100).

- **Formula:** Based on Halstead Volume, Cyclomatic Complexity, SLOC, and Comments.
- **Ranking:** A (>19), B (10-19), C (<10).

### 5.4 Cyclomatic Complexity Enhancements ✅

**Feature:** McCabe complexity with A-F ranking.

- **Ranks:** A (1-5), B (6-10), C (11-20), D (21-30), E (31-40), F (41+).

### 5.5 CLI Integration ✅

**Feature:** Full CLI parity with `radon` commands.

- `cytoscnpy cc`, `raw`, `hal`, `mi`
- Flags: `--average`, `--total-average`, `--min`, `--max`, `--json`, `--xml`.

### 5.6 Quality Gates & Failure Thresholds ✅

**Feature:** CI/CD integration with exit code 1 on failure.

- `cc --fail-threshold <N>`: Fail if complexity > N
- `mi --fail-under <N>`: Fail if MI < N
- `mi --average`: Show average MI
- `--fail-on-quality`: Integrated check in main analysis

---

## Phase 6: Editor Integration ✅ DONE

### 6.1 VS Code Extension ✅

- **Verification:** Verified extension code, compilation, and bundled binary.
- **File Switching:** Implemented `onDidChangeActiveTextEditor` to trigger analysis on tab switch.
- **Build Guide:** Created comprehensive guide for cross-platform builds.

---

## Phase 7: Infrastructure & Quality ✅ DONE

### 7.2 Error Handling ✅

**Problem:** Silently skipped files with syntax errors.
**Solution:**

- Implemented `ParseError` struct.
- Modified `analyzer.rs` to capture `rustpython_parser` errors.
- Updated `output.rs` to display a "Parse Errors" table.

---

## Phase 7.5: Performance Optimizations ✅ DONE

_Systematic performance improvements achieving 55% speed improvement._

### 7.5.1 Compiler Optimizations ✅

**Feature:** Aggressive release profile settings.

```toml
[profile.release]
lto = "thin"
codegen-units = 1
opt-level = 3
strip = true
```

**Impact:** ~15% performance improvement.
**Files:** `Cargo.toml`

### 7.5.2 Fast Hashing (FxHashMap) ✅

**Problem:** `std::collections::HashMap` uses SipHash (cryptographic, slower).

**Solution:** Replaced with `rustc-hash::FxHashMap` and `FxHashSet` throughout.

**Impact:** ~10-15% faster hash operations.
**Files:** `src/visitor.rs`, `src/analyzer/`

### 7.5.3 Reference Counting Optimization ✅

**Problem:** References stored as `Vec<(String, PathBuf)>` - PathBuf was never used.

**Solution:** Changed to `FxHashMap<String, usize>` for direct counting.

**Impact:** ~20% faster, 40-60% less memory.
**Files:** `src/visitor.rs`, `src/analyzer/processing.rs`

### 7.5.4 LineIndex Byte Iteration ✅

**Problem:** `char_indices()` iterates Unicode characters.

**Solution:** Use `as_bytes().iter()` since `\n` is always single byte in UTF-8.

**Impact:** ~5-10% faster LineIndex creation.
**Files:** `src/utils.rs`

### 7.5.5 Analyzer Module Refactor ✅

**Problem:** Monolithic `analyzer.rs` (1100+ lines).

**Solution:** Split into modular structure:

- `analyzer/mod.rs` - CytoScnPy struct + builders
- `analyzer/types.rs` - ParseError, AnalysisResult, AnalysisSummary
- `analyzer/heuristics.rs` - apply_penalties, apply_heuristics
- `analyzer/processing.rs` - Core processing methods

**Impact:** Improved maintainability, no performance regression.

### 7.5.6 lazy_static → OnceLock ✅

**Problem:** Using `lazy_static!` crate for static initialization.

**Solution:** Migrated to `std::sync::OnceLock` (Rust 1.70+).

**Impact:** Removes dependency, slightly faster initialization.
**Files:** `src/constants.rs`, `src/framework.rs`, `src/rules/secrets.rs`

### Performance Summary

| Stage                              | Time        | Improvement |
| ---------------------------------- | ----------- | ----------- |
| Baseline                           | 5.223 s     | -           |
| Phase 1 (LTO + FxHashMap)          | 4.044 s     | 22.6%       |
| Phase 2 (Reference counts)         | 3.059 s     | 41.4%       |
| **Phase 3 (LineIndex + OnceLock)** | **2.357 s** | **54.9%**   |

### 7.5.7 Additional Optimizations

_Status tracking for advanced optimizations identified in code review._

#### ✅ Completed Optimizations

| Optimization               | Description                                                                                                   | Files                         | Impact                            |
| -------------------------- | ------------------------------------------------------------------------------------------------------------- | ----------------------------- | --------------------------------- |
| **SmallVec for stacks**    | Stack-allocated vectors for `scope_stack`, `class_stack`, `function_stack`, `dataclass_stack`, `base_classes` | `visitor.rs`                  | Reduced heap allocations          |
| **Arc\<PathBuf\> sharing** | File paths shared via `Arc` instead of cloned `PathBuf`                                                       | `visitor.rs`                  | O(1) clone vs O(n)                |
| **Cached scope prefix**    | Maintains `cached_scope_prefix` string, updated incrementally on scope push/pop                               | `visitor.rs`                  | Avoids rebuilding qualified names |
| **Pre-allocated strings**  | Uses `String::with_capacity()` for known-size string building                                                 | `visitor.rs`, `processing.rs` | Fewer reallocations               |
| **Chunk-based Rayon**      | Processes files in chunks of 500 to prevent OOM on large projects                                             | `processing.rs`               | 85% memory reduction              |

#### 🔄 Pending Optimizations (Low Priority)

| Optimization                          | Description                                                              | Priority | Complexity | Est. Impact           |
| ------------------------------------- | ------------------------------------------------------------------------ | -------- | ---------- | --------------------- |
| **Reduce remaining `.clone()` calls** | Audit and eliminate unnecessary clones (~50+ remaining)                  | Low      | Low        | 2-5%                  |
| **String interning**                  | Use `string-cache` crate for repeated strings (module names, type names) | Low      | Medium     | 3-5%                  |
| **Profile-Guided Optimization (PGO)** | Build with profile data for 5-10% improvement                            | Low      | Medium     | 5-10%                 |
| **Parallel AST traversal**            | Split large files for parallel statement processing                      | Very Low | High       | 10-20% on large files |

#### ❌ Not Needed / Deferred

| Optimization                      | Reason                                                  |
| --------------------------------- | ------------------------------------------------------- |
| **Scope resolution caching**      | Already optimized via `cached_scope_prefix`             |
| **Arc\<String\> for module_name** | Would require significant refactoring, marginal benefit |
| **FxHashSet audit**               | Already using `FxHashSet` in most places                |

---

## Phase 7.6: Accuracy Improvements 🔄 IN PROGRESS

_Systematic improvements to detection accuracy based on benchmark analysis._

**Current Status:** F1 = 0.63 (77 TP, 34 FP, 60 FN)

### 7.6.1 Completed Fixes ✅

#### Return Type Annotation Tracking ✅

**Problem:** String annotations in return types were not being tracked:

```python
def get_data() -> "OrderedDict":  # "OrderedDict" was not tracked as a reference
    return {}
```

**Solution:** Added `visit_expr(node.returns)` for `FunctionDef` and `AsyncFunctionDef`.

**Files:** `src/visitor.rs`

#### TYPE_CHECKING Import Handling ✅

**Problem:** All TYPE_CHECKING imports were ignored, even genuinely unused ones:

```python
if TYPE_CHECKING:
    from typing import List  # Used in "List[str]" annotation - should be ignored ✅
    import json              # Never used - should be flagged ✅
```

**Solution:** Moved TYPE_CHECKING penalty from `apply_penalties()` to `apply_heuristics()` (runs after cross-file reference merge). Only suppresses imports with `references > 0`.

**Files:** `src/analyzer/heuristics.rs`

---

### 7.6.2 Remaining False Positives (34 items)

_Items incorrectly flagged as unused._

| Category      | Count | Issue                                                   | Priority | Fix Difficulty |
| ------------- | ----- | ------------------------------------------------------- | -------- | -------------- |
| **Functions** | 17    | Closures, returned functions, pattern matching bindings | High     | Medium         |
| **Imports**   | 6     | Cross-file `__all__` re-exports, FastAPI `Depends`      | High     | Medium         |
| **Variables** | 6     | Closure captures, complex scoping                       | Medium   | Hard           |
| **Methods**   | 3     | Pydantic `from_dict`/`to_dict` patterns                 | Low      | Easy           |
| **Classes**   | 2     | FastAPI response models (`In`, `Out`)                   | Low      | Easy           |

#### Priority 1: Cross-File `__all__` Tracking

**Problem:** Imports re-exported via `__all__` in other modules are flagged:

```python
# module_a.py
from module_b import ExportedClass  # Flagged as unused

# module_b.py
__all__ = ["ExportedClass"]  # Should mark as used across files
```

**Solution:** Track `__all__` exports globally and match against imports in other files.

#### Priority 2: Pattern Matching Bindings

**Problem:** Variables bound in `match` statements are flagged:

```python
match command:
    case (action, value):  # 'action' and 'value' flagged as unused
        handle(action, value)
```

**Solution:** Track `match` case bindings as references.

#### Priority 3: Returned Inner Functions

**Problem:** Functions returned from factory functions are flagged:

```python
def factory():
    def inner():  # Flagged as unused
        pass
    return inner  # Should mark 'inner' as used
```

**Status:** ✅ Fixed in return statement tracking improvements.

---

### 7.6.3 Remaining False Negatives (60 items)

_Genuinely unused items we fail to detect._

| Category      | Count | Issue                                           | Priority | Fix Difficulty    |
| ------------- | ----- | ----------------------------------------------- | -------- | ----------------- |
| **Functions** | 19    | Pragma-ignored, security examples, FastAPI deps | Low      | N/A (intentional) |
| **Variables** | 18    | Complex scoping, pattern matching, class attrs  | High     | Medium            |
| **Imports**   | 12    | Various tracking gaps                           | Medium   | Medium            |
| **Methods**   | 10    | Methods inside unused classes not linked        | High     | Medium            |
| **Classes**   | 1     | Complex inheritance patterns                    | Low      | Hard              |

#### Priority 1: Class-Method Linking

**Problem:** Methods inside unused classes are not detected:

```python
class UnusedClass:  # Detected as unused ✅
    def method(self):  # NOT detected (should be linked to class)
        pass
```

**Solution:** When a class is unused, automatically mark all its methods as unused.

#### Priority 2: Variable Scope Improvements

**Problem:** Local variables in complex scopes are missed:

```python
def func():
    x = 1  # Never used after assignment - should be flagged
    y = process()
    return y
```

**Solution:** Improve variable liveness analysis within function scopes.

#### Priority 3: Import Detection Gaps

**Problem:** Some import patterns not detected:

- Imports in type annotations without string quotes
- Imports used only in comprehensions
- Star imports (`from x import *`)

---

### 7.6.4 Accuracy Improvement Roadmap

| Phase     | Target F1 | Key Fixes                              | Status     |
| --------- | --------- | -------------------------------------- | ---------- |
| **7.6.1** | 0.63      | Return annotations, TYPE_CHECKING      | ✅ Done    |
| **7.6.2** | 0.68      | Cross-file `__all__`, pattern matching | 🔄 Planned |
| **7.6.3** | 0.72      | Class-method linking, variable scopes  | 🔄 Planned |
| **7.6.4** | 0.75      | Import gaps, framework patterns        | 🔄 Planned |

---

## Future Roadmap

### Phase 8: Advanced Framework Support

_Deepen understanding of popular Python frameworks to reduce false positives._

- [x] **Django Support** ✅

  - **URL Patterns:** Parse `urlpatterns` to find view functions referenced as strings.
  - **Admin:** Detect `admin.site.register(Model)` to mark models as used.
  - **Signals:** Detect `pre_save.connect(receiver)` to mark receivers.

- [x] **FastAPI Support** ✅

  - **Dependencies:** Scan `Depends(func)` in route handlers to mark dependency functions.

- [x] **Pydantic Support** ✅
  - **Field Tracking:** Explicitly track fields in `BaseModel` subclasses to avoid marking them as unused variables.

### Phase 9: Developer Experience

_Tools to improve the workflow around CytoScnPy._

- [x] **MCP Server (Model Context Protocol)**

  - Implemented `cytoscnpy-mcp` binary exposing CytoScnPy as MCP tools.
  - **Tools:** `analyze_path`, `analyze_code`, `cyclomatic_complexity`, `maintainability_index`
  - Stdio transport for Claude Desktop, Cursor IDE, and other MCP clients.
  - Usage: Add to `claude_desktop_config.json`:
    ```json
    { "mcpServers": { "cytoscnpy": { "command": "path/to/cytoscnpy-mcp" } } }
    ```

- [ ] **MCP HTTP/SSE Transport**

  - Add HTTP/SSE transport for remote LLM integrations (web-based clients, APIs).
  - **Challenges to Address:**
    - Path validation/sandboxing for security
    - Timeout handling for large project analysis (30-60s)
  - **Remote Analysis Tools:**
    | Tool | Input | Use Case |
    |------|-------|----------|
    | `analyze_code` | Code string | Small snippets (already works) |
    | `analyze_files` | JSON map of files | Medium projects via upload |
    | `analyze_repo` | Git URL | Clone & analyze public repos |
    | `analyze_path` | Local path | Server-local files only |
  - **Implementation:**
    - Add `--http --port 3000` CLI flags for transport selection
    - Use `rmcp` SSE transport feature
    - Add Git clone support for `analyze_repo` tool

- [ ] **LSP Server (Language Server Protocol)**

  - Implement a real-time LSP server for VS Code, Neovim, and Zed.
  - Provide instant diagnostics without saving or running CLI.

- [ ] **Git Integration**

  - **Blame Analysis:** Identify who introduced unused code.
  - **Incremental Analysis:** Analyze only files changed in the current PR/commit.

- [ ] **HTML Report Generation** _(NEW)_

  - Generate self-contained HTML reports for large codebase analysis.
  - **Features:**
    - Syntax highlighting (using highlight.js or prism.js)
    - Clickable file links with line numbers
    - Filtering by type (unused, security, quality), severity, file
    - Search across all findings
    - Summary dashboard with charts
    - Code snippets showing context around each finding
  - **CLI:**
    ```bash
    cytoscnpy analyze ./project --html report.html
    cytoscnpy analyze ./project --html-dir ./reports  # Multi-file for very large projects
    ```
  - **Implementation:**
    - Use `tera` or `askama` for templating
    - Embed CSS/JS for self-contained output
    - Optional: Split large reports into multiple HTML files with index

- [ ] **Live Server Mode** _(NEW)_

  - Built-in HTTP server to browse analysis results interactively.
  - **Features:**
    - Auto-refresh on file changes (watch mode)
    - REST API for findings (JSON endpoints)
    - Interactive code browser with inline annotations
    - Severity/type filters with live updates
  - **CLI:**
    ```bash
    cytoscnpy serve ./project --port 8080
    # Opens browser to http://localhost:8080
    # Watches for file changes and re-analyzes
    ```
  - **Technical Approach:**
    - Use `axum` or `warp` for lightweight HTTP server
    - WebSocket for live updates
    - Serve static HTML + JSON API
  - **Use Cases:**
    - Team code review sessions
    - CI/CD dashboard integration
    - Local development feedback loop

- [x] **Continuous Benchmarking**
  - Created benchmark suite with regression detection in `benchmark/`.

#### Benchmarking Infrastructure Ideas

| Component                   | Description                             | Tools/Approaches             |
| --------------------------- | --------------------------------------- | ---------------------------- |
| **Containerized Execution** | Isolated, reproducible environments     | Docker, Podman               |
| **Cross-Platform Matrix**   | Test on Windows, Linux, macOS           | GitHub Actions matrix        |
| **Python Version Matrix**   | Test with Python 3.8-3.12               | tox, nox                     |
| **Memory Profiling**        | Track peak RSS, allocations             | tracemalloc, memory_profiler |
| **CPU Profiling**           | Identify bottlenecks                    | py-spy, cProfile             |
| **Differential Testing**    | Compare outputs between tool versions   | Custom diff scripts          |
| **Regression Testing**      | Detect accuracy/performance regressions | Baseline JSON comparison     |

#### Suggested Future Improvements

1. **Expand Ground Truth**: Add more test cases for edge cases (decorators, type hints, async code)
2. **Real-World Validation**: Run on popular open-source projects (Django, Flask, requests)
3. **Add MCC Metric**: Better handles imbalanced detection categories
4. **Per-File Breakdown**: Show which specific test files each tool struggles with
5. **Confidence Threshold Sweep**: Test Vulture at multiple confidence levels (0%, 30%, 60%, 90%)
6. **Cross-Language Comparison**: Compare Python tools with similar tools for other languages

### Phase 10: Deep Analysis & Security

_Pushing the boundaries of static analysis._

- [x] **Taint Analysis**

  - Track data flow from user inputs (e.g., Flask `request.args`) to dangerous sinks (`eval`, `subprocess`, SQL).
  - Move beyond heuristic-based security checks.

- [x] **Secret Scanning 2.0**

  - Enhance regex scanning with entropy analysis to reduce false positives for API keys.

- [ ] **AST-Based Suspicious Variable Detection** _(Secret Scanning 3.0)_

  - **Problem:** Current regex patterns only detect secrets when the _value_ matches a known format (e.g., `ghp_xxx`). This misses hardcoded secrets assigned to suspiciously named variables:
    ```python
    database_password = "hunter2"        # Missed - no pattern match
    config['api_secret'] = some_value    # Missed - dict subscript
    ```
  - **Solution:** Leverage existing `CytoScnPyVisitor` AST traversal to detect assignments to suspicious variable names, regardless of the value format.
  - **Implementation:**

    ```rust
    // In visitor.rs - when visiting Assign nodes:
    const SUSPICIOUS_NAMES: &[&str] = &[
        "password", "secret", "key", "token", "auth", "credential",
        "api_key", "apikey", "private_key", "access_token", "pwd"
    ];

    fn matches_suspicious_name(name: &str) -> bool {
        let lower = name.to_lowercase();
        SUSPICIOUS_NAMES.iter().any(|s| lower.contains(s))
    }

    // When visiting an Assign node:
    if matches_suspicious_name(&target_name) {
        if let Some(string_value) = extract_string_value(&node.value) {
            findings.push(SecretFinding {
                message: format!("Suspicious assignment to '{}'", target_name),
                rule_id: "CSP-S300".to_owned(),
                file: file_path.clone(),
                line: node.range.start.row.get(),
                severity: "MEDIUM".to_owned(),
                matched_value: Some(redact_value(&string_value)),
                entropy: None,
            });
        }
    }
    ```

  - **Patterns to Detect:**
    - Simple assignments: `db_password = "secret123"`
    - Dict subscripts: `config['api_key'] = "token"`
    - Attribute assignments: `self.secret_key = "value"`
  - **False Positive Mitigation:**
    - Skip if value is `os.environ.get(...)` or `os.getenv(...)`
    - Skip if value references another variable (non-literal)
    - Skip if in test files (lower severity)
  - **Files:** `src/visitor.rs`, `src/rules/secrets.rs`
  - **New Rule ID:** `CSP-S300` (Suspicious Variable Assignment)

- [ ] **Modular Secret Recognition Engine** _(Secret Scanning 4.0)_

  - **Goal:** Refactor secret detection into a pluggable, trait-based architecture with unified context-based scoring.

  - **Architecture:**

    ```
    SecretScanner (Orchestrator)
           │
           ├── RegexRecognizer (built-in patterns)
           ├── AstRecognizer (variable name detection)
           ├── EntropyRecognizer (high-entropy strings)
           └── CustomRecognizer (user-defined via TOML)
                      │
                      ▼
              Context Scoring Engine
              (proximity, file type, pragma, dedup)
                      │
                      ▼
              Final Findings (scored & filtered)
    ```

  - **Pluggable Recognizers (Trait-based):**

    ```rust
    pub trait SecretRecognizer: Send + Sync {
        fn name(&self) -> &str;
        fn base_score(&self) -> u8;  // 0-100
        fn scan(&self, content: &str, line: usize) -> Vec<RawFinding>;
    }
    ```

  - **Context-Based Scoring Rules:**
    | Signal | Adjustment | Rationale |
    |--------|------------|-----------|
    | Near keyword (`api_key=`) | +20 | High confidence |
    | In test file | -50 | Likely fake |
    | In comment | -10 | Documentation |
    | High entropy | +15 | Random = suspicious |
    | Known FP pattern (URL/path) | -100 | Skip |
    | `os.environ.get()` | -100 | Not hardcoded |

  - **Configuration (TOML):**

    ```toml
    [secrets]
    min_score = 50  # Only report >= 50

    [secrets.recognizers.ast]
    suspicious_names = ["password", "secret", "key", "token"]

    [[secrets.custom_recognizers]]
    name = "Internal Token"
    regex = "INTERNAL_[A-Z0-9]{16}"
    score = 90
    ```

  - **Implementation Plan:**

    1. **Phase 1:** Add `confidence: u8` to `SecretFinding` struct
    2. **Phase 2:** Create `SecretRecognizer` trait in `src/rules/recognizers/mod.rs`
    3. **Phase 3:** Refactor existing patterns into `RegexRecognizer`
    4. **Phase 4:** Implement `AstRecognizer` (CSP-S300)
    5. **Phase 5:** Create `ContextScorer` with scoring rules
    6. **Phase 6:** Update `scan_secrets()` to use orchestrator pattern
    7. **Phase 7:** Add TOML config for custom recognizers

  - **Files:**
    - `src/rules/secrets.rs` → `src/rules/secrets/mod.rs` (split)
    - `src/rules/secrets/recognizers.rs` (new)
    - `src/rules/secrets/scoring.rs` (new)
    - `src/config.rs` (extend `SecretsConfig`)

- [x] **Type Inference (Lightweight)**

  - **Strategy:** Focus on fast, local, heuristic-based inference (e.g., literal tracking) to catch obvious errors (`str.append`).
  - **Non-Goal:** Do not attempt full constraint-based type solving (generics, cross-module). Leave that to dedicated tools like `mypy` or `ty`.
  - Basic inference for method misuse detection.

- [ ] **Dependency Graph**

  - Generate DOT/Mermaid graphs of module dependencies to aid refactoring.

- [ ] **License Compliance**
  - Scan `requirements.txt` and `Cargo.toml` for incompatible licenses.

### Phase 11: Auto-Remediation

_Safe, automated code fixes._

- [ ] **Safe Code Removal (`--fix`)**
  - **Challenge:** Standard AST parsers discard whitespace/comments.
  - **Strategy:** Use `RustPython` AST byte ranges or `tree-sitter` to identify ranges, then perform precise string manipulation to preserve formatting.

---

## 📦 Release Checklist

- [x] **Publish to PyPI** ✅

  - Verified `pyproject.toml` metadata.
  - Published via `maturin publish`.
  - Available: `pip install cytoscnpy`

- [ ] **Publish MCP Binary**
  - Build cross-platform binaries (Windows, Linux, macOS).
  - Create GitHub release with binaries.
  - Update Claude Desktop setup docs.
