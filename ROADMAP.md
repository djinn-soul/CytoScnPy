# CytoScnPy - Roadmap & Development Guide

> **Architecture:** Hybrid PyO3 + Standalone CLI
> **Status:** Production-ready core, active development

For completed features and implementation history, see [HISTORY.md](HISTORY.md).

---

## 📋 Table of Contents

1. [Project Status](#project-status)
2. [In Progress](#in-progress)
3. [Future Roadmap](#future-roadmap)
   - [Phase 9: Developer Experience](#phase-9)
   - [Phase 10: Deep Analysis & Security](#phase-10)
   - [Phase 11: Auto-Remediation](#phase-11)

---

## Project Status

## In Progress

### 5.7 Radon Parity Gaps 🔄 IN PROGRESS

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

## <a id="phase-6"></a>Phase 6: Editor Integration ✅ DONE

### 6.1 VS Code Extension ✅

### 6.2 Extension Code Audit (Pending Fixes) 🔄

#### 6.2.3 JSON Parsing Completeness ✅

_Fields in CLI JSON output not captured by `analyzer.ts`:_

- [ ] Add `summary` stats display in output channel

#### 6.2.4 Missing Commands 🔄

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

##### Priority 1: Class-Method Linking

**Problem:** Methods inside unused classes are not detected.

```python
class UnusedClass:  # Detected as unused ✅
    def method(self):  # NOT detected (should be linked to class)
        pass
```

**Solution:** When a class is unused, automatically mark all its methods as unused.

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
| **7.6.2** | 0.68      | Cross-file `__all__`, pattern matching | 🔄 Planned |
| **7.6.3** | 0.72      | Class-method linking, variable scopes  | 🔄 Planned |
| **7.6.4** | 0.75      | Import gaps, framework patterns        | 🔄 Planned |

---

## Future Roadmap

### <a id="phase-8"></a>Phase 8: Advanced Framework Support

Django, FastAPI, Pydantic is done ✅.

### <a id="phase-9"></a>Phase 9: Developer Experience

_Tools to improve the workflow around CytoScnPy._

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

- [ ] **HTML Report Generation**

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

    1. Add `confidence: u8` to `SecretFinding` struct
    2. Create `SecretRecognizer` trait in `src/rules/recognizers/mod.rs`
    3. Refactor existing patterns into `RegexRecognizer`
    4. Implement `AstRecognizer` (CSP-S300)
    5. Create `ContextScorer` with scoring rules
    6. Update `scan_secrets()` to use orchestrator pattern
    7. Add TOML config for custom recognizers

  - **Files:**
    - `src/rules/secrets.rs` → `src/rules/secrets/mod.rs` (split)
    - `src/rules/secrets/recognizers.rs` (new)
    - `src/rules/secrets/scoring.rs` (new)
    - `src/config.rs` (extend `SecretsConfig`)

- [ ] **Dependency Graph**

  - Generate DOT/Mermaid graphs of module dependencies to aid refactoring.

- [ ] **License Compliance**
  - Scan `requirements.txt` and `Cargo.toml` for incompatible licenses.

---

### <a id="phase-11"></a>Phase 11: Auto-Remediation

_Safe, automated code fixes._

- [ ] **Safe Code Removal (`--fix`)**
  - **Challenge:** Standard AST parsers discard whitespace/comments.
  - **Strategy:** Use `RustPython` AST byte ranges or `tree-sitter` to identify ranges, then perform precise string manipulation to preserve formatting.
