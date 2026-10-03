# CytoScnPy Architecture

CytoScnPy is a high-performance Python static analysis and codebase health assessment engine written in Rust with PyO3 bindings and standalone CLI binaries.

## Workspace Overview

The workspace is organized into three primary crates:

1. **`cytoscnpy` (Core Engine & Python Extension)**:
   - Contains all AST parsers, visitors, rule engines, graph analyzers, and scoring dimensions.
   - Compiles both as a Rust library crate and a Python C-extension via PyO3 and Maturin.
2. **`cytoscnpy-cli` (Command-Line Binary)**:
   - Standalone native executable providing subcommands (`score`, `deslop`, `functions`, `doctor`, `context`, `graph`, `quality`, etc.).
   - Includes progress spinners, formatted ASCII tables, JSON serialization, and CI gate exit codes.
3. **`cytoscnpy-mcp` (Model Context Protocol Server)**:
   - Implements JSON-RPC 2.0 MCP server over stdio for deep integration with AI coding assistants (e.g. Cursor, Claude Desktop, Antigravity).

## Core Architecture Pipelines

```
               Python Source Files (.py) & Repo Metadata
                                 │
                                 ▼
                    Path Normalization & Filters
                (utils/paths.rs, DEFAULT_EXCLUDE_FOLDERS)
                                 │
         ┌───────────────────────┼───────────────────────┐
         ▼                       ▼                       ▼
    Ruff Python AST          Git History              Doctor
  (ruff_python_parser)    (context/git_scanner)   (doctor/structure)
         │                       │                       │
         ├─ Functions & Metrics  ├─ Active/Frozen Files  ├─ Tooling & CI
         ├─ Singletons & Globals ├─ Churn Hotspots       ├─ Top-Level Dirs
         ├─ Side Effects & Todos └─ Token Estimates      └─ Test Ratios
         ├─ Clones & Duplicates
         └─ Module Graph & Cycles
                                 │
                                 ▼
                     10-Dimension Scoring Model
                    (scoring/dimensions/*.rs)
                                 │
                                 ▼
                    Prioritized Recommendations
                     (scoring/recommendations/)
                                 │
                                 ▼
               Output Formats: Terminal | JSON | LLM
```

### 1. Python AST Analysis
- **Parser**: Built on Ruff's ultra-fast Rust-native Python parser (`ruff_python_parser`).
- **Function Extractor** (`cytoscnpy::functions`): Extracts qualified names, line spans, McCabe cyclomatic complexity, and control-flow nesting depth. Excludes likely minified or bundled files automatically.
- **Runtime Smells**: Identifies mutable globals (`globals`), bare/empty exceptions (`exceptions`), import-time side effects (`side_effects`), singleton patterns (`singletons`), and anti-patterns such as magic numbers and deeply nested callbacks (`anti_patterns`).
- **Code Clones & Searchability**: Computes Type-1/2/3 duplicate clusters (`duplicates`) and detects cross-file filename collisions and generic function names (`searchability`).

### 2. Architecture & Module Graph
- **Canonical Module Identity**: Normalizes relative and absolute Python imports into canonical dot-separated module identities.
- **Coupling & Cycles**: Computes fan-in, fan-out, Tarjan's SCC circular import components, and cross-package layer boundaries (`architecture`).

### 3. Doctor & Repository Infrastructure
- **Tooling Detection** (`cytoscnpy::doctor`): Deep inspection of `pyproject.toml`, INI tool sections, `.pre-commit-config.yaml`, CI workflows, Dockerfiles, task runners, and documentation. It detects Python formatters, linters, type checkers, package lockfiles, CI providers, and Zed project settings, reporting each configured tool separately.
- **Structure Metrics**: Polyglot line/byte breakdowns, source-to-test ratios, directory depths, and top-level package inventories.

### 4. Git Context & Token Estimator
- **Code Churn Analysis** (`cytoscnpy::context`): Analyzes commit churn within a configurable window (default 30 days) to separate active from frozen files.
- **Context Pressure**: Identifies high-churn + high-complexity hotspots and projects LLM token consumption across reading, discovery, and dependency tracing phases.

### 5. Repository Health Scoring (DeSlop)
- **Slop Index (0–100)**: Evaluates 10 weighted dimensions: Setup Reliability, Architecture Clarity, Coupling / Blast Radius, Style Consistency, Test Safety Net, Runtime Predictability, Feedback Loop Speed, Documentation, Dependency Boundaries, Context Pressure.
- **Recommendation Engine**: Simulates prospective score reductions across candidate fixes, ranks recommendations by impact and effort, and formats actionable remediation plans for terminal or LLM consumption.
