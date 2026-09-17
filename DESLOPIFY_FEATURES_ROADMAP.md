# DeSlopify Features — CytoScnPy Implementation Checklist

> Features from the `deslopify` reference implementation that should be added to CytoScnPy. Mark an item complete only after implementation and relevant tests are finished.

## Product and CLI

- [ ] Provide a local, static LLM-friction/codebase-health analyzer with no cloud or LLM calls.
- [ ] Calculate a weighted Slop Index from 0 to 100.
- [ ] Show verdict bands: Clean, Acceptable, Messy, Sloppy, and Disaster.
- [ ] Support scanning the current directory and multiple supplied paths.
- [ ] Add `--format terminal`, `--format json`, and `--format llm`.
- [ ] Add `--verbose` for complete dimension output.
- [x] Add `--context-budget` for token-cost details.
- [ ] Add repeatable `--ignore` patterns.
- [x] Add Git controls: `--no-git` and `--git-months`.
- [ ] Add CI mode with `--ci` and an allowed-score gate using `--max-score`.

## Reports and integrations

- [ ] Create a readable terminal report with score, verdict, dimensions, evidence, and top fixes.
- [ ] Create a stable JSON report for automation and CI/CD.
- [ ] Create LLM-consumable, prioritized remediation instructions.
- [ ] Include score, raw score, verdict, size multiplier, and weighted dimension ratings in reports.
- [ ] Include repository totals, language breakdown, test/source counts, and detected configuration.
- [x] Include duplicate-code statistics.
- [x] Include active-surface and hot-file data when Git history is available.

## Scoring model

- [x] Score setup reliability from dependency manager, lockfile, Docker, build scripts, and `.gitignore`.
- [ ] Score architecture clarity from project depth, file sizes, organization, layering, and searchability.
- [ ] Score coupling/blast radius from import fan-in, fan-out, and cycles.
- [ ] Score style consistency from formatter, linter, and naming signals.
- [ ] Score test safety from test ratios, test files, and framework configuration.
- [ ] Score runtime predictability from global state, code smells, and runtime hazards.
- [ ] Score feedback-loop speed from build-time, test-runner, CI, and build-script signals.
- [ ] Score documentation from README quality, setup guidance, architecture docs, and contribution docs.
- [ ] Score dependency boundaries from ignore rules, lockfiles, and generated/vendor-code separation.
- [ ] Score context pressure from estimated tokens, function size, nesting, active surface, and dead code.

## Scanning and metadata

- [x] Respect repository and global Git ignore rules.
- [x] Skip dependency, generated, build, cache, editor, and virtual-environment directories.
- [x] Collect file, line, byte, largest-file, average-file, depth, and top-level-directory metrics.
- [x] Identify test files separately from source files.
- [x] Report per-language file, line, and byte totals.
- [x] Detect formatter, linter, type-checker, test, CI, Docker, dependency, lockfile, build, editor, documentation, and Git-ignore configuration.
- [x] Inspect Python project configuration for Ruff/Pylint, mypy/Pyright, and pytest.

## Language and AST analysis

- [ ] Detect supported source languages and common configuration/document formats.
- [ ] Recognize special filenames such as `Makefile` and `Dockerfile`.
- [ ] Add tree-sitter analysis for Python, JavaScript/JSX, TypeScript/TSX, Rust, Go, Java, C/C++, Ruby, and PHP.
- [ ] Extract function names, locations, length, cyclomatic complexity, and nesting depth.
- [ ] Calculate average and maximum function complexity, length, and nesting metrics.
- [ ] Exclude likely minified files from AST-oriented analysis.

## Architecture and searchability analysis

- [x] Build an import graph.
- [x] Calculate import fan-in and fan-out.
- [x] Detect circular-import components.
- [x] Detect bidirectional dependencies between module groups.
- [x] Detect god modules imported by most module groups.
- [x] Detect duplicate-code clusters and duplicate-line totals.
- [x] Detect naming-style distribution and consistency.
- [x] Detect duplicate filenames, colliding function names, and generic function names.
- [x] Detect potentially unreferenced large functions in files with no incoming imports.

## Quality and runtime analysis

- [x] Detect likely mutable global state.
- [x] Detect module-level side effects in Python and JavaScript/TypeScript.
- [x] Detect singleton patterns, mutable class variables, and global event listeners.
- [x] Detect TODO/FIXME/HACK/XXX placeholders.
- [x] Detect debug prints and commented-out code.
- [x] Detect bare exception/catch blocks and empty exception handlers.
- [x] Detect wildcard imports (`from module import *`).
- [x] Detect magic numbers and deeply nested callbacks.
- [x] Avoid context-sensitive false positives in tests and output-related files.

## Git-aware context analysis

- [x] Detect whether a scan target is a Git repository.
- [x] Identify active and frozen files from a configurable recent-history window.
- [x] Report commits, active files/lines/bytes, frozen files/bytes, and hot files.
- [x] Use the active code surface in context-pressure scoring.
- [x] Flag frequently changed files that are also complex.
- [x] Estimate token costs for discovery, reading, dependency tracing, and comprehension.
- [x] Report estimated navigation cost and context remaining for productive work.

## Recommendation engine

- [ ] Generate prioritized recommendations with estimated score reduction.
- [ ] Recommend splitting large logic-heavy files and simplifying complex functions.
- [x] Recommend adding tests, formatters, linters, and type checking.
- [x] Recommend README, architecture documentation, and CI configuration.
- [ ] Recommend resolving circular dependencies and enforcing module layering.
- [ ] Recommend removing anti-patterns and reducing mutable global state.
- [ ] Recommend extracting stable code into libraries to reduce active context surface.
- [ ] Recommend improving filename/function-name searchability.
- [ ] Recommend auditing potentially dead code.

---

## Detailed audit: DeSlopify compared with CytoScnPy

Audit date: 2026-09-13.

The checklist above copies DeSlopify capabilities, but many underlying Python analysis features already exist in CytoScnPy. The main additions are repository health scoring, Git activity analysis, context estimation, and ranked recommendations. Existing checklist boxes are preserved; use this audit to split existing capabilities from remaining integration work.

This is a source-level audit of all 73 checklist items, not a runtime validation. No tests were run for this comparison. “Existing” means an implemented capability, not exact output parity with DeSlopify. “Partial” means useful foundations exist but the complete roadmap requirement still needs work. “Missing” means no equivalent dedicated feature was found in the inspected code. “Scope expansion” identifies functionality beyond the current Python analyzer.

### 1. Product and CLI

| # | Roadmap item | Current status | Remaining work |
|---|---|---|---|
| 1 | Local static LLM-friction analyzer | Partial | Local static analysis exists; add repository-friction assessment. |
| 2 | Weighted Slop Index | Missing | Implement scoring, weights, normalization, and evidence. |
| 3 | Verdict bands | Missing | Add score-to-verdict mapping. |
| 4 | Current directory and multiple paths | Existing | Reuse existing path handling. |
| 5 | Terminal, JSON, LLM formats | Partial | Text and JSON exist; add health content and LLM instructions. |
| 6 | Verbose dimension output | Partial | Verbose diagnostics exist; add health-dimension details. |
| 7 | Context-budget flag | Missing | Add the estimator and CLI interface. |
| 8 | Repeatable ignore patterns | Partial | Folder exclusions exist; metric commands expose `--ignore`. Define consistent filtering for health analysis. |
| 9 | Git controls | Missing | Add history enable/disable and lookback configuration. |
| 10 | CI and maximum-score gate | Partial | CI gates exist; add a health-score threshold. |

CytoScnPy supports text, JSON, JUnit, GitHub, GitLab, Markdown, and SARIF output. Preserve `text`; `terminal` could be an alias if needed. Source: [CLI options](cytoscnpy/src/cli/options.rs).

### 2. Reports and integrations

| # | Roadmap item | Current status | Remaining work |
|---|---|---|---|
| 11 | Terminal score, dimensions, top fixes | Partial | Reuse output infrastructure; add the health report model. |
| 12 | JSON reporting | Existing foundation | Define a health schema and compatibility tests over existing serialization. |
| 13 | Prioritized LLM remediation instructions | Missing | Add a renderer over structured recommendations. |
| 14 | Score, raw score, multiplier, dimensions | Missing | Add health-specific result fields. |
| 15 | Repository/language/test/config summary | Partial | Existing file, line, size, directory, and metric totals need language/config inventories and test ratios. |
| 16 | Duplicate statistics | Partial | Clone results and statistics exist; add repository health aggregates. |
| 17 | Active surface and hot-file reporting | Missing | Depends on Git-history analysis. |

Existing results include per-file metrics, clones, average complexity, maintainability, raw metrics, and Halstead metrics. Sources: [result types](cytoscnpy/src/analyzer/types.rs), [clone statistics](cytoscnpy/src/commands/clones/stats.rs).

### 3. Scoring model

All ten repository scoring dimensions are new, but several measurements are reusable.

| # | Dimension | Reusable CytoScnPy inputs | Missing inputs or logic |
|---|---|---|---|
| 18 | Setup reliability | Existing (`cytoscnpy doctor`) | Setup reliability 0-100 scoring, tooling inventory, lockfile/CI/docker/test/linter checks |
| 19 | Architecture clarity | File metrics, complexity, class cohesion | Directory organization, layering, searchability scoring |
| 20 | Coupling/blast radius | Import bindings and references | Module graph statistics, cycle reports, scoring |
| 21 | Style consistency | Python definitions | Naming distribution and formatter/linter detection |
| 22 | Test safety | Test identification and inclusion policy | Test/source ratios, framework inventory, scoring |
| 23 | Runtime predictability | Quality findings, bare-except detection | Dedicated global-state and import-time hazard analysis |
| 24 | Feedback-loop speed | Existing integration infrastructure | Inspect the target project's tooling and estimate feedback cost |
| 25 | Documentation | No equivalent health assessment found | README/setup/architecture/contribution checks |
| 26 | Dependency boundaries | Missing/unused dependencies and development dependencies used in production | Vendor/generated separation, lockfile signals, scoring |
| 27 | Context pressure | Complexity, length, nesting, dead-code findings | Token estimates, active surface, aggregation, scoring |

CytoScnPy's own CI integration does not imply detection of CI configuration in scanned repositories. Maintainability Index is not equivalent to Slop Index. Sources: [analysis metrics](cytoscnpy/src/analyzer/types.rs), [reference dimensions](deslopify/src/scoring/dimensions.rs).

### 4. Scanning and metadata

| # | Roadmap item | Current status | Remaining work |
|---|---|---|---|
| 28 | Repository/global Git ignore | Existing (`cytoscnpy doctor`) | Gitignore-aware repo walker respects ignore rules. |
| 29 | Skip dependency/build/cache directories | Existing (`cytoscnpy doctor`) | Standard filters and directory pruning skips build, cache, venv, and git dirs. |
| 30 | File/line/byte/depth statistics | Existing (`cytoscnpy doctor`) | Total files, lines, bytes, average lines, max depth, largest file. |
| 31 | Identify tests separately | Existing (`cytoscnpy doctor`) | Source vs test file counts, line counts, and test-to-source ratios. |
| 32 | Per-language totals | Existing (`cytoscnpy doctor`) | Polyglot extension scanner counts files, lines, and bytes per language. |
| 33 | Detect project tooling/configuration | Existing (`cytoscnpy doctor`) | Formatters, linters, type checkers, tests, CI, Docker, lockfiles, build scripts. |
| 34 | Ruff/Pylint/mypy/Pyright/pytest settings | Existing (`cytoscnpy doctor`) | Deep inspection of `pyproject.toml` tool tables and build backend. |

The scanner already respects `.gitignore`, global ignore, and `.git/info/exclude`, and filters excluded directories during traversal. Test inventory must count tests even when normal analysis excludes them. Source: [scanner](cytoscnpy/src/utils/paths.rs).

### 5. Language and AST analysis

| # | Roadmap item | Current status | Remaining work |
|---|---|---|---|
| 35 | Multiple languages/config formats | Scope expansion | Separate lightweight inventory from full language analysis. |
| 36 | Makefile/Dockerfile recognition | Missing as health inventory | Read as setup metadata. |
| 37 | Multilanguage tree-sitter analysis | Scope expansion | CytoScnPy uses Ruff's Python AST; broader parsing needs a separate scope decision. |
| 38 | Function names, locations, length, complexity, nesting | Existing Python foundation | Normalize health metrics; exact per-function nesting export may need adding. |
| 39 | Average/maximum function metrics | Partial | Average complexity exists; complete length/nesting aggregates need verification and extension. |
| 40 | Skip minified source | Missing | Primarily relevant to JavaScript/multilanguage expansion. |

Repository scoring does not require replacing the existing Python parser. Sources: [complexity rules](cytoscnpy/src/rules/quality/complexity.rs), [length and nesting rules](cytoscnpy/src/rules/quality/maintainability.rs).

### 6. Architecture and searchability

| # | Roadmap item | Current status | Remaining work |
|---|---|---|---|
| 41 | Import graph | Existing (`cytoscnpy graph`) | Normalized module graph with canonical module identity resolution. |
| 42 | Fan-in/fan-out | Existing (`cytoscnpy graph`) | Computed from resolved internal and external module edges. |
| 43 | Circular-import components | Existing (`cytoscnpy graph`) | Tarjan's SCC cycle detector and readable cycle paths. |
| 44 | Bidirectional group dependencies | Existing (`cytoscnpy graph`) | Top-level package group aggregation and cross-boundary cycle detection. |
| 45 | God modules | Existing (`cytoscnpy graph`) | Documented high coupling / incoming dependency concentration heuristic. |
| 46 | Duplicate clusters and line totals | Existing (`cytoscnpy duplicates`) | Code clone clustering (Type-1/2/3), interval union algorithm for non-overlapping physical duplicate line totals per file and project-wide, duplication percentage, CLI subcommand (`cytoscnpy duplicates`, aliases `dupes`, `clones-summary`), JSON report, and unified `deslop` integration with CI gates (`max_duplicate_clusters`, `max_duplicate_lines`, `max_duplicate_pct`). |
| 47 | Naming-style distribution | Existing (`cytoscnpy naming`) | Python-aware identifier naming distribution, consistency percentage, dominant style badge, structural dunder exemptions, private/mangled prefix handling, terminal and JSON reports, and unified `deslop` gate integration. |
| 48 | Duplicate filenames/function names | Existing (`cytoscnpy searchability`) | Codebase searchability: duplicate filenames across directories (excluding package `__init__.py`), function name collisions (defined in 3+ distinct files, excluding structural dunders and fixtures), generic identifier detection, terminal report, JSON export, and CI gating. |
| 49 | Unreferenced large functions | Existing (`cytoscnpy unreferenced`) | Detection of large functions (15+ lines, >=8 char specific names) in isolated Python files (zero incoming imports/references across codebase), framework prefix and test filtering, CLI subcommand (`cytoscnpy unreferenced`, aliases `dead-functions`, `isolated-functions`), JSON export, and unified `deslop` integration with CI gates (`max_unreferenced_functions`, `max_unreferenced_lines`). |

The import-binding graph used for re-export reference propagation is not automatically an architecture graph. DeSlopify's dead-code detection uses isolation, text/name matching, size thresholds, and broad framework-name exemptions; reuse CytoScnPy's definitions, references, exports, and framework handling. Sources: [visitor state](cytoscnpy/src/visitor/state.rs), [reference dead-code heuristic](deslopify/src/analysis/dead_code.rs).

### 7. Quality and runtime analysis

| # | Roadmap item | Current status | Remaining work |
|---|---|---|---|
| 50 | Mutable global state | Existing (`cytoscnpy globals`) | Python AST module collections, class variables, and `global` mutations; Rust `static mut` and JS/TS top-level mutables; CLI subcommand and `deslop` gate. |
| 51 | Module-level side effects | Existing (`cytoscnpy side-effects`) | Python AST module-level import-time side-effect detection (top-level function calls, loops, and with-statements, exempting safe logging/warnings/guards and entrypoints); polyglot JS/TS top-level network/server calls and global event listeners; CLI subcommand (`--json`, `--fail-on-any`, `--max-side-effects`), unified deslop report, and `max_side_effects` gate. |
| 52 | Singletons/class mutables/event listeners | Existing (`cytoscnpy singletons`, `cytoscnpy globals`, `cytoscnpy side-effects`) | Python AST-based singleton pattern detection (overridden `__new__` instance caching, `_instance` class attribute with `get_instance()` accessor, `@singleton` decorators, and `metaclass=Singleton`), CLI subcommand (`--json`, `--fail-on-any`, `--max-singletons`, alias `singleton`), unified deslop report, and `max_singletons` gate. Mutable class variables covered by `cytoscnpy globals`, event listeners covered by `cytoscnpy side-effects`. |
| 53 | TODO/FIXME/HACK/XXX | Existing (`cytoscnpy todos`) | Fast line-by-line annotation scanner for TODO, FIXME, HACK, XXX with token boundary checks. |
| 54 | Debug prints/commented-out code | Existing (`cytoscnpy todos`) | Debug print statement detection (`print(`, `console.log(`, `println!(`, etc.) and commented-out code blocks (`# if`, `# def`, etc.). |
| 55 | Bare exceptions and empty handlers | Existing (`cytoscnpy exceptions`) | Python AST-based bare-except and empty-handler detection; handlers with `pass`, `...`, or bare string bodies flagged as empty; recursive detection in functions, class methods, nested try/for/while/with/match; CLI subcommand with `--json`, `--fail-on-any`, `--max-bare-excepts`, `--max-empty-handlers`; gates in `cytoscnpy deslop` and `[tool.cytoscnpy.deslop]`. |
| 56 | Wildcards/magic numbers/nested callbacks | Existing (`cytoscnpy wildcards`, `cytoscnpy anti-patterns`) | Wildcard import detection via AST scanner (`cytoscnpy wildcards`); magic number and deeply nested callback detection via Python AST & indentation analyzer (`cytoscnpy anti-patterns`, aliases `antipatterns`, `magic-numbers`, `callbacks`), unified deslop report, and gates `max_anti_patterns`, `max_magic_numbers`, `max_nested_callbacks`. |
| 57 | Test/output false-positive handling | Existing (`cytoscnpy todos`) | Context-sensitive suppression skipping debug print detection in test files and output-oriented modules (`cli`, `main`, `output`, `views`, etc.). |

Mutable default arguments differ from mutable class variables. Recognizing wildcard imports for reference resolution does not mean reporting them as quality problems. Sources: [best-practice rules](cytoscnpy/src/rules/quality/best_practices.rs), [global usage rule](cytoscnpy/src/rules/quality/performance/global_usage.rs).

### 8. Git activity and context

| # | Roadmap item | Current status | Remaining work |
|---|---|---|---|
| 58 | Detect Git repository | Existing (`cytoscnpy context`) | Validates `.git` and `git rev-parse --git-dir` with graceful non-git fallback. |
| 59 | Active/frozen files | Existing (`cytoscnpy context`) | Classifies by recent commit churn in lookback window. |
| 60 | Commit/activity/hot-file report | Existing (`cytoscnpy context`) | Collects and exposes commit totals, active/frozen line/byte ratios, and top hot files. |
| 61 | Active surface in scoring | Existing (`cytoscnpy context`) | Biases context reading and navigation models toward active code surface. |
| 62 | Frequently changed and complex files | Existing (`cytoscnpy context`) | Cross-references churn with AST cyclomatic complexity to flag risk hotspots. |
| 63 | Discovery/reading/tracing/comprehension tokens | Existing (`cytoscnpy context`) | Implements Python-aware token estimators across all navigation phases. |
| 64 | Navigation cost and remaining context | Existing (`cytoscnpy context`) | Configurable `--context-budget`, navigation percentage, and qualitative verdict. |

Git ignore handling and Git-history analysis are separate capabilities. DeSlopify defaults to a maximum one-month lookback, adapts the window to repository age, and retains ten hot files. Source: [reference Git analysis](deslopify/src/scanner/git.rs).

### 9. Recommendations

| # | Roadmap item | Current status | Remaining work |
|---|---|---|---|
| 65 | Prioritized score-reduction recommendations | Missing | Add simulation, ranking, and recommendation schema. |
| 66 | Split files/simplify functions | Partial | Turn existing length/complexity findings into repository recommendations. |
| 67 | Tests/formatters/linters/type checking | Missing | Depends on tooling and test inventories. |
| 68 | README/architecture docs/CI | Missing | Depends on repository configuration inspection. |
| 69 | Resolve cycles/enforce layering | Missing | Depends on architecture analysis. |
| 70 | Anti-patterns/global state | Partial | Aggregate existing and new findings into health recommendations. |
| 71 | Extract stable libraries | Missing | Depends on activity and architecture metrics. |
| 72 | Improve searchability | Missing | Depends on filename/name-collision analysis. |
| 73 | Audit dead code | Existing foundation | Add health impact and ranking to existing findings and fixes. |

Reuse contextual clone-refactoring suggestions. DeSlopify sorts recommendations by estimated reduction and returns at most ten. Sources: [clone suggestions](cytoscnpy/src/commands/clones/suggestions.rs), [reference ranking](deslopify/src/recommendations/mod.rs).

### 10. Reference behaviors requiring qualification

| Finding | Implication |
|---|---|
| Final score includes a size multiplier | Small repositories can have low final scores despite poor raw ratings; report both. |
| Fixed 176,000-token usable context | Make capacity configurable and display the assumption. |
| Build time estimated from lines and language | This is not a measured build or test duration. |
| README setup detection uses keywords | Keyword presence does not prove instructions work. |
| Lockfile freshness uses modification times | Checkout/copy timestamps do not prove dependency consistency. |
| Test safety uses file ratios | This is not coverage, test quality, or passing-test verification. |
| Custom ignores use trimmed substring matching | The reference does not implement general glob semantics for these strings. |
| Duplicate detection uses overlapping eight-line windows | Summed duplicate lines are not necessarily unique duplicated LOC. |
| Runtime/anti-pattern checks use broad heuristics | The empty-catch regex also matches Python exception header lines. |
| Recommendation reductions are simulated individually | Improvements affecting the same dimension can overlap and must not be blindly summed. |
| Potential dead-code counts appear in context-pressure evidence | Those counts do not directly increase that dimension's rating; distinguish evidence from score contribution. |

Sources: [score formula](deslopify/src/scoring/mod.rs), [context estimator](deslopify/src/scoring/context_budget.rs), [setup/build heuristics](deslopify/src/scanner/quality.rs), [ignore handling](deslopify/src/scanner/walker.rs), [duplication](deslopify/src/analysis/duplication.rs), [anti-patterns](deslopify/src/analysis/patterns.rs), [dimension implementation](deslopify/src/scoring/dimensions.rs).

### 11. Recommended implementation order

- [ ] Normalize existing Python function, file, clone, dead-code, and test-inventory metrics. Do not derive all metrics solely from threshold-exceeding findings.
- [ ] Add repository metadata inspection for tooling, documentation, lockfiles, and directory structure.
- [ ] Define a health-report model separating raw measurements, assumptions, dimension ratings, and final score.
- [ ] Add Python module architecture analysis with resolved module identities before fan-in/out, cycles, and layering.
- [ ] Add optional Git activity handling missing Git, shallow history, multiple roots, and changed/untracked files deliberately.
- [ ] Add context estimates and recommendations with configurable capacity and without double-counting projected improvements.
- [ ] Extend CLI, JSON, and CI gates while preserving existing output names and failure behavior.
- [ ] Evaluate full multilanguage parsing separately from lightweight language inventory.

### 12. Suggested checkbox structure

Split compound tasks into existing capabilities and unfinished integration work before marking them complete. Example:

- [x] Support text and JSON output.
- [ ] Add repository health fields to text and JSON reports.
- [ ] Add LLM-oriented remediation output.
- [x] Detect Python bare-except blocks.
- [x] Detect empty exception handlers.
- [x] Add import-time side-effect checks.
- [x] Detect Python dead code across modules.
- [ ] Include dead-code metrics in repository health reports.
- [ ] Rank dead-code remediation alongside other health recommendations.
