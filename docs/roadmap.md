# CytoScnPy - Roadmap & Development Guide

> **Architecture:** Hybrid PyO3 + Standalone CLI
> **Status:** Production-ready core, active development

For completed features and implementation history, see [GitHub Releases](https://github.com/djinn09/CytoScnPy/releases).

---

## Table of Contents

1. [Project Status](#project-status)
2. [In Progress](#in-progress)
3. [Future Roadmap](#future-roadmap)
   - [Phase 9: Developer Experience](#phase-9)
   - [Phase 10: Deep Analysis & Security](#phase-10)
   - [Phase 11: Auto-Remediation](#phase-11)

---

## Project Status

The PyO3-based CLI, bundled `cytoscnpy-cli` binary, and `cytoscnpy-mcp` server together deliver the production-ready analysis stack described below: quality, secret, and clone reporting (see `docs/CLI.md`, `docs/usage.md`, and `README.md`) are exercised across platforms, while the VS Code extension and HTML reporting surface that data to editors and browsers.

Core safeguards—quality gates (`--fail-on-quality`, `--fail-threshold`), security scanning, `--fix`/`--apply` auto-remediation, and stdin/stdout-based MCP hosting—are stable and shipping; the roadmap now focuses on higher-accuracy analysis and better user experience on top of that foundation.

## In Progress

The sections below highlight the work that is still active:

- **Phase 5.7 (Radon Parity Gaps)** – The parity tests around module-level complexity, `else` clauses on loops/try, wildcard matching, and Halstead/raw metrics are in place, but the analyzer logic still needs to be implemented (see the `### 5.7` section below).
- **Phase 6 (Editor Integration)** – The VS Code extension and accompanying code audit continue to evolve; Phase 6.1 and 6.2 list the UX, command, and bundling gaps that remain.
- **Phase 7.6 (Accuracy Improvements)** – The benchmark (F1 = 0.72) and the remaining false positives/negatives (34/60 items) are being chipped away in the dedicated Phase 7.6 subsection.

### 5.7 Radon Parity Gaps IN PROGRESS

**Status:** Tests added, implementation pending. See `cytoscnpy/tests/radon_parity_*.rs`

These features are tested but not yet implemented. Remove `#[ignore]` from tests when implementing.

#### Complexity Gaps (19 tests ignored)

| Feature                     | Description                           | Test File                         | Radon Behavior                                     |
| --------------------------- | ------------------------------------- | --------------------------------- | -------------------------------------------------- |
| **Module-level complexity** | Complexity of code outside functions  | `radon_parity_complexity_test.rs` | Radon reports module-level `if`/`for`/`while` etc. |
| **For/while else clause**   | `else:` on loops adds +1 complexity   | `radon_parity_complexity_test.rs` | Radon counts loop `else:` as branch                |
| **Try-except else clause**  | `else:` on try adds +1 complexity     | `radon_parity_complexity_test.rs` | Radon counts try `else:` as branch                 |
| **Lambda ternary**          | Ternary inside lambda adds complexity | `radon_parity_complexity_test.rs` | Ternary in lambda body counts                      |
| **Ternary with generator**  | Generator inside ternary              | `radon_parity_complexity_test.rs` | Nested comprehension complexity                    |
| **Match wildcard**          | `case _:` shouldn't add complexity    | `radon_parity_complexity_test.rs` | Wildcard is default, not branch                    |
| **Nested generator**        | Inner generator adds complexity       | `radon_parity_complexity_test.rs` | Each `for`/`if` in nested generator                |
| **Class method `or`**       | Boolean `or` in condition             | `radon_parity_complexity_test.rs` | `or` adds +1 complexity                            |

#### Halstead Gaps (1 test ignored)

| Feature                       | Description                | Test File                       | Radon Behavior               |
| ----------------------------- | -------------------------- | ------------------------------- | ---------------------------- |
| **Distinct operand counting** | `if a and b: elif b or c:` | `radon_parity_halstead_test.rs` | `b` counted once as distinct |

#### Raw Metrics Gaps (2 tests ignored)

| Feature                            | Description                  | Test File                  | Radon Behavior                 |
| ---------------------------------- | ---------------------------- | -------------------------- | ------------------------------ |
| **Line continuation with string**  | Backslash + multiline string | `radon_parity_raw_test.rs` | Continuation counted correctly |
| **Line continuation with comment** | Backslash + inline comment   | `radon_parity_raw_test.rs` | Comment on continuation line   |

#### Implementation Priority

1. **Module-level complexity** - High impact (8 tests), required for full Radon parity
2. **Loop/try else clauses** - Medium impact (5 tests), common pattern
3. **Match wildcard handling** - Low impact (2 tests), Python 3.10+ only
4. **Halstead/Raw edge cases** - Low impact (3 tests), edge cases

---

## <a id="phase-8"></a>Phase 8: CFG Integration ✅ DONE

### 8.1 Control Flow Graph Construction ✅

Implemented CFG construction from Python AST for behavioral analysis:

- **CFG Builder**: Constructs basic blocks from `StmtFunctionDef`
- **Control Flow**: Handles `if`, `for`, `while`, `try`, `match`, `break`, `continue`, `return`, `raise`
- **Loop Depth**: Tracks nesting depth for each basic block
- **Fingerprinting**: Behavioral signature for clone comparison

### 8.2 Clone Detection Integration ✅

CFG validation as secondary filter for clone detection:

| Feature                  | Description                                    |
| ------------------------ | ---------------------------------------------- |
| `cfg_validation` config  | Enable/disable CFG validation in `CloneConfig` |
| `validate_with_cfg()`    | Phase 4.5 filter in `CloneDetector::detect()`  |
| `cfg_validated` context  | +15 confidence boost in `ConfidenceScorer`     |
| 70% similarity threshold | CFG pairs below this are filtered out          |

### 8.3 Feature Flag

Enabled via `--features cfg` at compile time:

```bash
cargo build --features cfg
cargo test --features cfg
```

---

## <a id="phase-6"></a>Phase 6: Editor Integration ✅ DONE

### 6.1 VS Code Extension IN PROGRESS

### 6.2 Extension Code Audit (Pending Fixes) IN PROGRESS

#### 6.2.3 JSON Parsing Completeness ✅

_Fields in CLI JSON output not captured by `analyzer.ts`:_

- [ ] Add `summary` stats display in output channel

#### 6.2.4 Missing Commands

| Command                   | Description                     | Status |
| ------------------------- | ------------------------------- | ------ |
| `cytoscnpy.taintAnalysis` | Run taint analysis specifically | ❌     |

#### 6.2.5 Path Handling ✅

- [ ] Add macOS (`cytoscnpy-cli-darwin`) binary bundling
- [ ] Add Linux (`cytoscnpy-cli-linux`) binary bundling

#### 6.2.6 UX Enhancements

| Feature            | Description                                | Priority | Status |
| ------------------ | ------------------------------------------ | -------- | ------ |
| Status Bar         | Show finding count in status bar           | Medium   | ❌     |
| Sidebar Badge      | Show issue count in Explorer sidebar       | Medium   | ✅     |
| Quick Fixes        | Code actions to remove/comment unused code | High     | ✅     |
| Gutter Decorations | Visual icons for severity levels           | Low      | ✅     |
| Progress Indicator | Show progress during workspace analysis    | Medium   | ❌     |
| File Caching       | Skip re-analyzing unchanged files          | Low      | ✅     |
| Problem Grouping   | Better categorization in Problems panel    | Low      | ✅     |

---

## <a id="phase-7"></a>Phase 7: Infrastructure & Quality ✅ DONE

### 7.2 Error Handling ✅

### 7.3 Parser Migration: `rustpython-parser` → `ruff_python_parser` ✅

**Reference:** See [RustPython/Cargo.toml](https://github.com/RustPython/RustPython/blob/main/Cargo.toml) for working example.

---

## Phase 7.5: Performance Optimizations ✅ DONE

#### Pending Optimizations (Low Priority)

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

## Phase 7.6: Accuracy Improvements IN PROGRESS

_Systematic improvements to detection accuracy based on benchmark analysis._

**Current Status:** F1 = 0.72 (110 TP, 46 FP, 38 FN)

### 7.6.1 Completed Fixes ✅

- [x] **Framework Decorator Tracking:** Accurate detection for FastAPI, Django, and Celery entry points.
- [x] **TYPE_CHECKING Block Handling:** Correctly ignores imports used only in type-check blocks.
- [x] **F-string Reference Detection:** Tracking variables and functions referenced within f-string interpolations.
- [x] **Multi-line String LOC:** Improved metrics for backslash-continued strings and comments.

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

##### Priority 1: Class-Method Linking ✅ DONE

**Problem:** Methods inside unused classes are not detected.

```python
class UnusedClass:  # Detected as unused ✅
    def method(self):  # NOW detected via cascading detection ✅
        pass
```

**Solution:** When a class is unused, automatically mark all its methods as unused (cascading deadness).

**Implementation:** Modified `aggregate_results()` and `analyze_code()` in `processing.rs` to flag all methods within unused classes. Respects heuristic protections (visitor pattern methods are excluded).

##### Priority 2: Variable Scope Improvements

**Problem:** Local variables in complex scopes are missed.

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
| **7.6.2** | 0.68      | Cross-file `__all__`, pattern matching | PLANNED |
| **7.6.3** | 0.72      | Class-method linking, variable scopes  | PLANNED |
| **7.6.4** | 0.75      | Import gaps, framework patterns        | PLANNED |

---

### <a id="phase-8-advanced"></a>Phase 8: Advanced Framework Support ✅ DONE

Django, FastAPI, Pydantic is done ✅.

## Future Roadmap

### <a id="phase-9"></a>Phase 9: Developer Experience

_Tools to improve the workflow around CytoScnPy._

- [x] **Git Hooks (pre-commit)** ✅
  - Automated analysis on commit/push.
  - See `docs/pre-commit.md` for setup instructions.
- [x] **CI/CD Integration Examples** ✅
  - Reference workflows for GitHub Actions provided in `.github/workflows/`.
  - Supports `--fail-on-quality` and `--fail-threshold` for gatekeeping.
- [x] **uv Package Manager Integration** ✅
  - Full support for `uv`-managed environments.
  - Used in official lint/CI workflows.

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

- [ ] **Config File Support for Notebook Options**
  - Allow `include_ipynb` and `ipynb_cells` in `.cytoscnpy.toml` and `pyproject.toml`
  - Currently these are CLI-only flags (`--include-ipynb`, `--ipynb-cells`)
  - **Rationale:** Enable persistent configuration without passing flags on every run
  - **Implementation:** Add fields to `CytoScnPyConfig` struct in `src/config.rs`

- [ ] **Git Integration**
  - **Blame Analysis:** Identify who introduced unused code.
  - **Incremental Analysis:** Analyze only files changed in the current PR/commit.

- [x] **HTML Report Generation** ✅
  - Generate self-contained HTML reports for large codebase analysis.
  - **Features:**
    - Syntax highlighting (using highlight.js or prism.js)
    - Clickable file links with line numbers
    - Filtering by type (unused, security, quality), severity, file
    - Search across all findings
    - Summary dashboard with charts
    - Code snippets showing context around each finding (Basic impl done, see Phase 9.5 for improvements)
  - **CLI:**
    ```bash
    cytoscnpy ./project --html
    # Multi-file support planned for large projects
    ```
  - **Implementation:**
    - Use `tera` or `askama` for templating
    - Embed CSS/JS for self-contained output
    - Optional: Split large reports into multiple HTML files with index

- [x] **Security Documentation Overhaul** ✅
  - Categorized all 50+ danger rules into logical modules (Code Execution, Injection, etc.).
  - Ensured 1:1 parity between documentation and Rust implementation (severities, patterns).
  - Added safer alternatives and remediation advice for all rules.
  - See [Dangerous Code Rules](dangerous-code.md) for details.

- [ ] **Live Server Mode**
  - Built-in HTTP server to browse analysis results interactively.
  - **Features:**
    - Auto-refresh on file changes (watch mode)
    - REST API for findings (JSON endpoints)
    - Interactive code browser with inline annotations
    - Severity/type filters with live updates
  - **CLI:**
    ```bash
    cytoscnpy serve ./project --port 8080
    ```
  - **Technical Approach:**
    - Use `axum` or `warp` for lightweight HTTP server
    - WebSocket for live updates
    - Serve static HTML + JSON API
  - **Use Cases:**
    - Team code review sessions
    - CI/CD dashboard integration
    - Local development feedback loop
  - **Smoke-test reference:** `cytoscnpy-mcp/scripts/test_mcp_server.py` drives `cytoscnpy mcp-server` over JSON-RPC (initialize, tools/list, analyze_code) so you can validate the CLI-hosted MCP transport before wiring it into downstream clients.

### <a id="phase-9-5"></a>Phase 9.5: Report Actionability Upgrade PLANNED

_Implementing findings from the Recommendation System Audit._

**Goal:** Transform the report from a diagnostic tool into a remediation platform.

- [ ] **Remediation Display Engine** (Priority: HIGH)
  - **Problem:** Backend has remediation data (e.g., "Use parameterized queries"), but it's lost during report generation.
  - **Solution:**
    - Extend `IssueItem` struct with `remediation` and `vuln_type` fields.
    - Update `flatten_issues` to preserve `SinkInfo` remediation strings.
    - Update `issues.html` and `file_view.html` to display a collapsible "Remediation" box.

- [ ] **Context-Aware Code Snippets** (Priority: MEDIUM)
  - **Problem:** Issues are shown as one-liners without context.
  - **Solution:**
    - Extract 3-5 lines of code around the issue location.
    - Display syntax-highlighted snippets inline in the Issues tab.

- [ ] **Enriched Quality Messages** (Priority: MEDIUM)
  - **Problem:** Generic messages like "Function too complex" offer no guidance.
  - **Solution:** Map rule IDs to specific refactoring advice (e.g., "Extract reusable logic into helper functions").

- [ ] **Prioritization Framework** (Priority: LOW)
  - **Problem:** All high-severity issues look the same.
  - **Solution:** Add "Exploitability" and "Fix Effort" scores to help teams prioritize.

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

---

### <a id="phase-10"></a>Phase 10: Deep Analysis & Security

_Pushing the boundaries of static analysis._

- [x] **Secret Scanning 2.0**
  - Enhance regex scanning with entropy analysis to reduce false positives for API keys.

- [x] **AST-Based Suspicious Variable Detection** _(Secret Scanning 3.0)_ ✅
  - `AstRecognizer` now walks assignments, annotations, attributes, and dict subscripts, emits `CSP-S300` when suspicious names (password, secret, key, token, etc.) are assigned literal strings, and skips env vars, placeholders, and test files while lowering confidence (`cytoscnpy/src/rules/secrets/recognizers.rs:296-438`).

- [x] **Modular Secret Recognition Engine** _(Secret Scanning 4.0)_ ✅
  - `SecretScanner` orchestrates `RegexRecognizer`, `AstRecognizer`, `EntropyRecognizer`, and optional `CustomRecognizer`, feeds their raw matches to the `ContextScorer`, deduplicates findings by line, and filters by `SecretsConfig` thresholds like `min_score`, `entropy_threshold`, and `suspicious_names` (`cytoscnpy/src/rules/secrets/mod.rs:1-138`, `cytoscnpy/src/config.rs:88-157`, `cytoscnpy/src/rules/secrets/scoring/mod.rs:1-147`).

  - `ContextScorer` applies bonuses/penalties for keywords, entropy, comments, docstrings, placeholders, and env-var patterns before clamping to 0-100, so the modular engine already enforces the scoring rules described in the previous plan (`cytoscnpy/src/rules/secrets/scoring/mod.rs:1-147`).


- [ ] **Dependency Graph** IN PROGRESS
  - Generate DOT/Mermaid graphs of module dependencies to aid refactoring.
  - Core `CallGraph` infrastructure implemented in `cytoscnpy/src/taint/call_graph.rs`.

- [ ] **License Compliance**
  - Scan `requirements.txt` and `Cargo.toml` for incompatible licenses.

---

### <a id="phase-11"></a>Phase 11: Auto-Remediation ✅ DONE

_Safe, automated code fixes._

- [x] **Safe Code Removal (`--fix`)**
  - **Implementation:** Use AST byte ranges from `ruff_python_parser` for precise removal.
  - **Features:**
    - `--fix` flag removes unused functions, classes, and imports
    - `--dry-run` previews changes without applying
    - CST mode (tree-sitter) is now enabled by default for better comment preservation
    - Only high-confidence items (≥90%) are auto-fixed
    - Cascading detection: methods inside unused classes are auto-removed with their parent class

---

### <a id="phase-12"></a>Phase 12: Security & Lifecycle

- [ ] **Fuzzing Environment Stabilization**
  - Fuzzing is currently difficult on Windows due to MSVC toolchain and sanitizer runtime issues.
  - **Solution:** Transition fuzzing CI to a purely Linux-based environment (or WSL).
  - This allows reliable `cargo fuzz` execution to catch edge-case crashes and undefined behavior.
  - **Implementation:** Add a `fuzz-linux.yml` workflow that runs in Ubuntu and uses `cargo +nightly fuzz`.

---

### <a id="phase-13"></a>Phase 13: Interprocedural Taint Analysis

_Deep data-flow analysis across function boundaries._

- [ ] **Global Call Graph Construction** IN PROGRESS
  - Map function calls across the entire project to track how data moves between modules.
  - Necessary for tracking "taint" from a source in one file to a sink in another.
  - **Status:** `cytoscnpy/src/taint/call_graph.rs` already builds nodes, callee/caller edges, and qualifier handling; remaining work is propagating taint/sanitization through that graph.
- [ ] **Cross-Function Taint Tracking**
  - Store and propagate "taint state" for function arguments and return values.
  - **Goal:** Catch vulnerabilities like an API request being passed through a helper function into an `eval()` or SQL query.
- [ ] **Sanitization Recognition**
  - Detect when tainted data passes through "safe" functions (like `html.escape()` or custom sanitizers).
  - **Benefit:** Significantly reduces False Positives by knowing when data is no longer dangerous.
- [ ] **Framework-Specific Entry Points**
  - Add deep support for FastAPI dependencies, Django middleware, and Flask request hooks.
  - **Benefit:** Provides "Premium" level security coverage for modern Python web applications.

---

_135 total ground truth items, 11 tools benchmarked_
