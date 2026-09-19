# CLI Reference

This document provides a comprehensive reference for the `cytoscnpy` command-line interface.

## Command Syntax

```bash
cytoscnpy [OPTIONS] [COMMAND]
```

## Main Options

### Input & Output

- `[paths]`: One or more paths to analyze (files or directories). If omitted, CytoScnPy defaults to the current working directory.
- `--root <PATH>`: Explicitly sets the project root. This is **highly recommended for CI/CD environments** to ensure that path-based security containment is correctly applied and that relative imports are resolved consistently. When `--root` is used, positional `[paths]` are not allowed. It also ensures that reports (SARIF, GitLab, etc.) use paths relative to this root.
- `--format <FORMAT>`: Specifies the output format. Supported values: `text` (default), `json`, `junit`, `github`, `gitlab`, `markdown`, `sarif`, `grouped` (deprecated).
- `--json`: Format the result as a raw JSON object. Shorthand for `--format json`. This is ideal for piping into tools like `jq` or for consumption by CI/CD scripts.
- `--verbose`, `-v`: Prints detailed logs during the analysis process, including which files are being scanned and any non-fatal issues encountered.
- `--quiet`: Minimalist output. Only the final summary table (or JSON) is displayed, suppressing the per-file findings table.
- `--fail-on-any`: Enables all supported failure gates and exits with code `1` if any actionable finding is detected. For unused code, it uses zero tolerance unless `--fail-threshold`, config, or `CYTOSCNPY_FAIL_THRESHOLD` supplies a threshold.
- `--fail-on-quality`: Causes the process to exit with code `1` if _any_ code quality issues (like high complexity or deep nesting) are detected.
- `--fail-on-secrets`: Enables secret scanning if needed and exits with code `1` if any secret findings are detected.
- `--fail-on-danger`: Enables dangerous-code/taint scanning if needed and exits with code `1` if any danger or taint findings are detected.
- `--fail-on-missing-deps`: Enables dependency analysis if needed and exits with code `1` if any missing dependency findings are detected.
- `--fail-on-unused-deps`: Enables dependency analysis if needed and exits with code `1` if any unused dependency findings are detected.
- `--html`: Generates a self-contained, interactive HTML report. Note that this feature may require additional dependencies and automatically enables quality scanning.
- `--client <CLIENT>`: Identify the calling editor/client. Currently only `vscode` is supported. When `vscode` is set, project config from `.cytoscnpy.toml` or `pyproject.toml` is still honored, and explicit VS Code settings are passed as CLI flags that override matching thresholds.

### Scan Types

- `--secrets`, `-s`: Actively scans for high-entropy strings, API keys, and hardcoded credentials. It checks variables, strings, and even comments (depending on configuration).
- `--danger`, `-d`: Enables security scanning for dangerous patterns like `eval()`, `exec()`, and insecure temporary file creation. It also activates **taint analysis** to track user-controlled data flowing into dangerous sinks (e.g., SQL injection or command injection points). See [Dangerous Code](dangerous-code.md) for a full rule list.
- `--quality`, `-q`: Runs code quality checks including Cyclomatic Complexity, Maintainability Index, block nesting depth, and function length/argument counts.
- `--no-dead`, `-n`: Skips the default dead code detection. Use this if you only care about security vulnerabilities or quality metrics and want to speed up the analysis.
- `--deps`: Analyze unused and missing dependencies by cross-referencing `pyproject.toml` / `requirements.txt` against imports found in the code. Opt-in — not run by default.

### Analysis Configuration

- `--confidence <N>`, `-c`: Sets a minimum confidence threshold (0-100). CytoScnPy uses a scoring system for dead code; setting this to `80`, for example, will suppress "noisy" findings where the tool isn't certain the code is unused.
- `--exclude-folders <DIRS>`: Exclude specific folders from analysis. Can be used multiple times.
- `--include-folders <DIRS>`: Force-include specific folders in analysis. Can be used multiple times.
- `--include-tests`: By default, CytoScnPy excludes `tests/`, `test/`, `test_*.py`, `*_test.py`, `conftest.py`, and `noxfile.py` from main analysis, metrics, file statistics, and clone detection. Use this flag to include them. Dependency analysis still scans test/dev files intentionally so it can distinguish development-only imports from production imports.
- `--include-ipynb`: Enables scanning of Jupyter Notebook files. CytoScnPy extracts the Python code from cells and analyzes it as a virtual module.
- `--ipynb-cells`: When combined with `--include-ipynb`, this reports findings with cell numbers instead of just line numbers, making it easier to locate issues in the Notebook UI.
- `--clones`: Activates **duplicate code detection**. It uses AST-based hashing to find code blocks that are identical or nearly identical across your codebase.
- `--clone-similarity <N>`: Sets the similarity threshold for clone detection (0.0 to 1.0). A value of `1.0` finds only exact duplicates; a lower value like `0.8` (default) finds similar logic that might be refactored.
- `--fix`: Enables "Dead Code Auto-Fix" mode. By default, this is a **dry-run**—it will show you exactly what code would be removed without touching your files.
- `--apply`, `-a`: Executes the changes suggested by `--fix`. **Warning: This modifies your source code.** It is highly recommended to run with `--fix` first to review changes, and to have a clean Git state before applying.
  - `--fix` targets: functions, methods, classes, imports, and unused variables.
  - `--fix` always enforces a minimum confidence floor of **80%** for safety, even if `--confidence` is set lower.
  - In dry-run mode with `--json`, CytoScnPy emits a deterministic JSON fix plan (`kind: "dead_code_fix_plan"`) suitable for editor/CI consumption.
  - When removing the only method in a class, CytoScnPy inserts `pass` to keep valid Python syntax.
- `--make-whitelist`: Generates a Python whitelist from currently detected unused symbols.
- `--whitelist <PATH>`: Loads one or more whitelist files to suppress matching dead-code findings.

### CI/CD Failure Gates

These flags allow you to set strict gates for CI/CD. If any enabled gate fails, CytoScnPy exits with code `1`. Failure gates that depend on optional scans enable those scans automatically.

- `--fail-on-any`: Convenience gate for CI. Implies quality, secrets, danger/taint, missing dependency, and unused dependency failure gates. For unused code, it defaults to `--fail-threshold 0.0` unless an explicit threshold is supplied.
- `--fail-threshold <N>`: Exit with 1 if the total percentage of unused code exceeds `N`.
- `--max-complexity <N>`: Sets the maximum allowed Cyclomatic Complexity (standard is often `10`).
- `--min-mi <N>`: Sets the minimum allowed Maintainability Index (usually `40-65`).
- `--max-nesting <N>`: Sets the maximum allowed indentation/nesting level (e.g., `3` or `4`).
- `--max-args <N>`: Sets the maximum number of arguments a function can have.
- `--max-lines <N>`: Sets the maximum number of lines a function can have.
- `--fail-on-quality`: Exit with 1 if any quality issue is found.
- `--fail-on-secrets`: Exit with 1 if any secret finding is found; implies `--secrets`.
- `--fail-on-danger`: Exit with 1 if any danger or taint finding is found; implies `--danger`.
- `--fail-on-missing-deps`: Exit with 1 if missing dependencies are found; implies `--deps`.
- `--fail-on-unused-deps`: Exit with 1 if unused dependencies are found; implies `--deps`.

## Subcommands

### `raw`

Calculate raw metrics (LOC, LLOC, SLOC, Comments, Multi, Blank).

```bash
cytoscnpy raw [OPTIONS] <PATH>
```

- `-j`, `--json`: Output JSON.
- `-s`, `--summary`: Show summary metrics.
- `-O`, `--output-file <FILE>`: Save output to file.
- `-e`, `--exclude <DIR>`: Folders to exclude.
- `-i`, `--ignore <PATTERN>`: Glob patterns to ignore.

### `cc`

Calculate Cyclomatic Complexity.

```bash
cytoscnpy cc [OPTIONS] <PATH>
```

- `-a`, `--average`: Show average complexity.
- `--total-average`: Show total average complexity.
- `-s`, `--show-complexity`: Show complexity score with rank.
- `-n`, `--min <RANK>`: Set minimum complexity rank (A-F).
- `-x`, `--max <RANK>`: Set maximum complexity rank (A-F).
- `-o`, `--order <ORDER>`: Ordering function (score, lines, alpha).
- `--no-assert`: Do not count assert statements.
- `--xml`: Output XML.
- `--fail-threshold <N>`: Exit 1 if any block has complexity > N.
- `-j`, `--json`: Output JSON.
- `-e`, `--exclude <DIR>`: Folders to exclude.
- `-i`, `--ignore <PATTERN>`: Glob patterns to ignore.
- `-O`, `--output-file <FILE>`: Save output to file.

### `hal`

Calculate Halstead Metrics.

```bash
cytoscnpy hal [OPTIONS] <PATH>
```

- `-f`, `--functions`: Compute metrics on function level.
- `-j`, `--json`: Output JSON.
- `-e`, `--exclude <DIR>`: Folders to exclude.
- `-i`, `--ignore <PATTERN>`: Glob patterns to ignore.
- `-O`, `--output-file <FILE>`: Save output to file.

### `mi`

Calculate Maintainability Index.

```bash
cytoscnpy mi [OPTIONS] <PATH>
```

- `-s`, `--show`: Show actual MI value.
- `-a`, `--average`: Show average MI.
- `-n`, `--min <RANK>`: Set minimum MI rank (A-C).
- `-x`, `--max <RANK>`: Set maximum MI rank (A-C).
- `--multi`: Count multiline strings as comments (default: true).
- `--fail-threshold <N>`: Exit 1 if any file has MI < N.
- `-j`, `--json`: Output JSON.
- `-e`, `--exclude <DIR>`: Folders to exclude.
- `-i`, `--ignore <PATTERN>`: Glob patterns to ignore.
- `-O`, `--output-file <FILE>`: Save output to file.

### `stats`

Generate comprehensive project statistics report.

```bash
cytoscnpy stats [OPTIONS] <PATH>
```

- `-a`, `--all`: Enable all analysis: secrets, danger, quality, files.
- `--root <PATH>`: Project root for analysis (use instead of positional path).
- `-s`, `--secrets`: Scan for secrets.
- `-d`, `--danger`: Scan for dangerous code.
- `-q`, `--quality`: Scan for quality issues.
- `-j`, `--json`: Output JSON.
- `-o`, `--output <FILE>`: Output file path.
- `--exclude-folders <DIRS>`: Exclude specific folders from analysis.

### `deslop`

Run the DeSlopify-derived architecture, Git context, and repository-health
analyzers and return one report. Existing CytoScnPy source analyzers such as
dead code, clones, dependencies, secrets, danger/taint, and quality remain on
their existing commands and are not included here.

```bash
cytoscnpy deslop [OPTIONS] <PATH>
```

- `--fail-on-any`: Exit with code `1` when any configured architecture,
  hotspot, context-budget, or health gate fails. The global form
  (`cytoscnpy --fail-on-any deslop ...`) is also supported.
- `--root <PATH>`: Project root for analysis (use instead of positional path).
- `--json`: Output one JSON report.
- `-o`, `--output <FILE>`: Output report file.
- `--exclude <DIR>`: Exclude a folder or path pattern from analysis.
- `--no-git`: Disable Git history analysis.
- `--git-months <N>`: Set the maximum Git lookback.
- `--context-budget <TOKENS>`: Set usable LLM context capacity.

The JSON result has stable top-level sections: `architecture`, `context`,
`health`, `searchability`, `naming`, `todos`, and `gates`, plus `schema_version`.

```bash
cytoscnpy deslop . --json
cytoscnpy deslop . --json --fail-on-any
```

Gate limits can be set under `[cytoscnpy.deslop]` (or
`[tool.cytoscnpy.deslop]`):

```toml
max_cycles = 0
max_god_modules = 0
max_hotspots = 0
min_health_score = 70
# max_navigation_pct = 75.0
# max_duplicate_filenames = 0
# max_function_collisions = 0
# min_naming_consistency = 0.85
# max_todos = 0
```

### `searchability`

Analyze codebase searchability, name collisions, and generic identifiers:

- Detect duplicate filenames across directories (excluding package `__init__.py`).
- Detect function and method name collisions across distinct files (defined in 3+ files, excluding Python dunders, test fixtures, and structural methods).
- Detect generic, low-information filenames and function names (`utils`, `helpers`, `common`, `handler`, `process`, etc.).

```bash
cytoscnpy searchability [OPTIONS] [PATHS]...
```

- `--json`: Output structured JSON report with `stats`, `duplicate_files`, `function_collisions`, and `generic_names`.
- `--fail-on-collisions`: Exit with code `1` if any function collisions (defined in >= 3 distinct files) are detected.
- `--fail-on-duplicates`: Exit with code `1` if any duplicate filenames are detected.
- `--fail-on-any`: Exit with code `1` if any searchability issue (duplicate filenames or function collisions) is detected.
- `-o`, `--output-file <FILE>`: Save report to file.
- `--exclude <DIRS>`: Exclude folders from searchability analysis.

### `naming`

Analyze Python identifier naming style distribution and consistency:

- Classifies function and method identifiers into `snake_case`, `camelCase`, `PascalCase`, `SCREAMING_SNAKE_CASE`, or `mixed`.
- Python-aware rules: trims leading private/mangled underscores (`_private`, `__mangled`), trims trailing keyword collision underscores (`class_`), exempts structural dunder methods (`__init__`) and unittest fixtures (`setUp`), and ignores anonymous/lambda functions.
- Calculates dominant style, distribution breakdown, consistency ratio (0-100%), and identifies non-conforming outliers with source locations.

```bash
cytoscnpy naming [OPTIONS] [PATHS]...
```

- `--json`: Output structured JSON report with `stats` and `outliers`.
- `--min-consistency <RATIO>`: Minimum consistency ratio required (e.g. `0.85` or `85.0`).
- `--fail-on-inconsistent`: Exit with code `1` if naming consistency falls below threshold (default: 0.85 or `--min-consistency`).
- `--fail-on-any`: Exit with code `1` if naming consistency falls below threshold (alias for `--fail-on-inconsistent`).
- `-o`, `--output-file <FILE>`: Save report to file.
- `--exclude <DIRS>`: Exclude folders from naming analysis.

### `todos`

Detect `TODO`, `FIXME`, `HACK`, and `XXX` annotations, debug print statements, and commented-out code blocks:

- Scans source files with word-boundary awareness for annotation markers.
- Detects debug prints (`print(`, `console.log(`, `println!(`, `puts `, `dbg!(`, etc.) while automatically suppressing them in test files and output-oriented directories (`cli`, `main`, `output`, `views`, etc.).
- Detects commented-out code blocks (`# if`, `# def`, `// for`, etc.).
- Outputs human-readable terminal summary or machine-readable JSON.

```bash
cytoscnpy todos [OPTIONS] [PATHS]...
```

- `--json`: Output structured JSON report with `stats` and `matches`.
- `--fail-on-any`: Exit with code `1` when any annotation or debug print is detected.
- `-o`, `--output-file <FILE>`: Save report to file.
- `--exclude <DIRS>`: Exclude folders or patterns from scan.

### `globals`

Detect mutable global state across Python and polyglot source files:

- Detects module-level mutable data structures (`list`, `dict`, `set`, `defaultdict`, `deque`, `Counter`, etc.).
- Detects class-level mutable variables shared across all instances.
- Detects functions mutating module globals via `global`.
- Polyglot detection for Rust `static mut` and JavaScript/TypeScript top-level mutable `var` and collection `let` declarations.
- Automatically suppresses test files (`tests/`, `test_*.py`, `conftest.py`, etc.).
- Outputs human-readable terminal summary or machine-readable JSON.

```bash
cytoscnpy globals [OPTIONS] [PATHS]...
```

- `--json`: Output structured JSON report with `stats` and `matches`.
- `--fail-on-any`: Exit with code `1` when any mutable global state is detected.
- `--max-globals <N>`: Fail if total mutable globals exceed `N`.
- `-o`, `--output-file <FILE>`: Save report to file.
- `--exclude <DIRS>`: Exclude folders or patterns from scan.

### `exceptions`

Detect bare-except and empty exception-handler anti-patterns (alias `bare-except`):

- Detects `except:` statements with no specific exception type (`BareExcept`).
- Detects handlers whose bodies only contain `pass`, `...`, or bare string comments (`EmptyHandler`).
- Recursively scans modules, functions, classes, loops, with-blocks, and match statements.
- Outputs human-readable terminal summary or machine-readable JSON.

```bash
cytoscnpy exceptions [OPTIONS] [PATHS]...
# Or alias
cytoscnpy bare-except [OPTIONS] [PATHS]...
```

- `--json`: Output structured JSON report with `stats` and `matches`.
- `--fail-on-any`: Exit with code `1` when any exception anti-pattern is detected.
- `--max-bare-excepts <N>`: Fail if total bare-except blocks exceed `N`.
- `--max-empty-handlers <N>`: Fail if total empty exception handlers exceed `N`.
- `-o`, `--output-file <FILE>`: Save report to file.
- `--exclude <DIRS>`: Exclude folders or patterns from scan.

### `wildcards`

Detect wildcard imports across Python source files (alias `star-imports`):

- Detects `from <module> import *` statements polluting module or function namespaces.
- Recursively inspects module level and nested block scopes.
- Captures the imported module name (including relative dots like `.`, `..`).
- Outputs human-readable terminal summary or machine-readable JSON.

```bash
cytoscnpy wildcards [OPTIONS] [PATHS]...
# Or alias
cytoscnpy star-imports [OPTIONS] [PATHS]...
```

- `--json`: Output structured JSON report with `stats` and `matches`.
- `--fail-on-any`: Exit with code `1` when any wildcard import is detected.
- `--max-wildcards <N>`: Fail if total wildcard imports exceed `N`.
- `-o`, `--output-file <FILE>`: Save report to file.
- `--exclude <DIRS>`: Exclude folders or patterns from scan.

### `side-effects`

Detect module-level import-time side effects:

- Detects top-level function calls, loops (`for`/`while`), and `with` statements executed on import.
- Automatically exempts safe logging/warnings setup and entrypoint files (`setup.py`, `conftest.py`, etc.).
- Detects polyglot JavaScript/TypeScript top-level network/server calls and global event listeners.
- Outputs human-readable terminal summary or machine-readable JSON.

```bash
cytoscnpy side-effects [OPTIONS] [PATHS]...
```

- `--json`: Output structured JSON report with `stats` and `matches`.
- `--fail-on-any`: Exit with code `1` when any import-time side effect is detected.
- `--max-side-effects <N>`: Fail if total module-level side effects exceed `N`.
- `-o`, `--output-file <FILE>`: Save report to file.
- `--exclude <DIRS>`: Exclude folders or patterns from scan.

### `singletons`

Detect Python singleton patterns (alias `singleton`):

- Detects instance caching in overridden `__new__` methods.
- Detects `_instance` class attribute caches paired with `get_instance()` / `getInstance()` accessors.
- Detects `@singleton` / `@Singleton` class decorators.
- Detects `metaclass=Singleton` class declarations.
- Outputs human-readable terminal summary or machine-readable JSON.

```bash
cytoscnpy singletons [OPTIONS] [PATHS]...
# Or alias
cytoscnpy singleton [OPTIONS] [PATHS]...
```

- `--json`: Output structured JSON report with `stats` and `matches`.
- `--fail-on-any`: Exit with code `1` when any singleton pattern is detected.
- `--max-singletons <N>`: Fail if total singleton patterns exceed `N`.
- `-o`, `--output-file <FILE>`: Save report to file.
- `--exclude <DIRS>`: Exclude folders or patterns from scan.

### `anti-patterns`

Detect code-quality anti-patterns (magic numbers and deeply nested callbacks; aliases `antipatterns`, `magic-numbers`, `callbacks`):

- Detects hardcoded magic numbers (3+ digits) in comparisons, conditional tests, and loop expressions (exempting module/class level constant declarations like `MAX_SIZE = 500`).
- Detects deeply nested callbacks, closures, lambdas, or control structures nested 4+ levels deep (>= 16 spaces indentation).
- Automatically suppresses test files (`test_*.py`, `conftest.py`, `tests/`).
- Outputs human-readable terminal summary or machine-readable JSON.

```bash
cytoscnpy anti-patterns [OPTIONS] [PATHS]...
# Or aliases
cytoscnpy antipatterns [OPTIONS] [PATHS]...
cytoscnpy magic-numbers [OPTIONS] [PATHS]...
cytoscnpy callbacks [OPTIONS] [PATHS]...
```

- `--json`: Output structured JSON report with `stats` and `matches`.
- `--fail-on-any`: Exit with code `1` when any anti-pattern is detected.
- `--max-anti-patterns <N>`: Fail if total anti-patterns exceed `N`.
- `--max-magic-numbers <N>`: Fail if magic numbers exceed `N`.
- `--max-nested-callbacks <N>`: Fail if deeply nested callbacks exceed `N`.
- `-o`, `--output-file <FILE>`: Save report to file.
- `--exclude <DIRS>`: Exclude folders or patterns from scan.

### `duplicates`

Analyze duplicate-code clusters and non-overlapping duplicate-line totals (aliases: `dupes`, `clones-summary`):

- Groups duplicate code fragments into clusters across Python source files (Type-1 exact, Type-2 renamed, Type-3 similar).
- Computes non-overlapping physical duplicate lines per file and project-wide using interval unions to eliminate double-counting.
- Reports duplication percentage, top affected files, and cluster code locations.
- Supports CI gates on cluster count, total duplicate lines, and duplication percentage.

```bash
cytoscnpy duplicates [OPTIONS] [PATHS]...
# Or aliases
cytoscnpy dupes [OPTIONS] [PATHS]...
cytoscnpy clones-summary [OPTIONS] [PATHS]...
```

- `--json`: Output structured JSON report with `clusters`, `stats`, and `file_stats`.
- `--fail-on-any`: Exit with code `1` when any duplicate code cluster is detected.
- `--max-clusters <N>`: Fail if total duplicate clusters exceed `N`.
- `--max-duplicate-lines <N>`: Fail if total non-overlapping duplicate lines exceed `N`.
- `--max-duplicate-pct <PCT>`: Fail if duplicate line percentage exceeds `PCT` (e.g. `5.0`).
- `--min-similarity <VAL>`: Similarity threshold (0.0 - 1.0, default: `0.85`).
- `--min-lines <N>`: Minimum line threshold for code fragments (default: `4`).
- `--include-tests`: Include test files in duplication analysis (excluded by default).
- `-o`, `--output-file <FILE>`: Save report to file.
- `--exclude <DIRS>`: Exclude folders or patterns from scan.

### `unreferenced`

Detect potentially unreferenced large functions in files with no incoming imports (aliases: `dead-functions`, `isolated-functions`):

- Identifies isolated Python files that have no incoming imports or incoming references from any other files in the project.
- Scans functions in isolated files for large implementations (15+ lines by default) with specific non-trivial names.
- Filters out short helper names (<8 chars), test functions, dunder methods, and common framework hook prefixes (`get`, `set`, `on`, `handle`, `render`, `validate`, `resolve`, etc.).
- Verifies that candidate functions are not referenced anywhere else across the codebase.
- Supports CI gates on unreferenced function count and total unreferenced lines.

```bash
cytoscnpy unreferenced [OPTIONS] [PATHS]...
# Or aliases
cytoscnpy dead-functions [OPTIONS] [PATHS]...
cytoscnpy isolated-functions [OPTIONS] [PATHS]...
```

- `--json`: Output structured JSON report with `items`, `isolated_files`, and `stats`.
- `--fail-on-any`: Exit with code `1` when any unreferenced large function is detected.
- `--max-unreferenced <N>`: Fail if total unreferenced functions exceed `N`.
- `--max-unreferenced-lines <N>`: Fail if total unreferenced lines exceed `N`.
- `--min-lines <N>`: Minimum line threshold for large functions (default: `15`).
- `--include-tests`: Include test files in analysis (excluded by default).
- `-o`, `--output-file <FILE>`: Save report to file.
- `--exclude <DIRS>`: Exclude folders or patterns from scan.

### `score`

Calculate the repository Weighted Slop Index (0–100), Verdict band, context size multiplier, and the 10-dimension health breakdown.

```bash
cytoscnpy score [OPTIONS] [PATHS]...
# Or aliases
cytoscnpy slop-index [OPTIONS] [PATHS]...
cytoscnpy slop [OPTIONS] [PATHS]...
```

- `--json`: Output structured JSON report including `slop_index`, `raw_score`, `size_multiplier`, `verdict`, `dimensions`, and `recommendations`.
- `--format <FORMAT>`: Output format: `terminal` (default ASCII summary table with top fixes), `json`, or `llm` (markdown prompt instructions for AI coding agents).
- `--ci`: CI mode. Exits with code `1` if `slop_index` exceeds `--max-score` (default max score threshold: `50.0`).
- `--max-score <N>`: Maximum allowable Slop Index before failing the gate.
- `--context-budget <TOKENS>`: Effective LLM context window in tokens (default: `176000`) for active-surface scaling.
- `--no-git`: Disable Git commit history analysis (uses full byte count rather than active surface).
- `--git-months <N>`: Lookback window in months for Git active-surface detection (default: `1`).
- `-o`, `--output <FILE>`: Save report to file.
- `--exclude <DIRS>`: Exclude folders or patterns from analysis.

#### Verdict Bands

| Band | Slop Index Range | Description |
|---|---|---|
| **Clean** | 0 – 20 | Excellent health, negligible LLM friction |
| **Acceptable** | 21 – 40 | Normal codebase with minor slop within reasonable boundaries |
| **Messy** | 41 – 60 | Moderate structural friction, refactoring recommended |
| **Sloppy** | 61 – 80 | High friction, difficult navigation and maintenance |
| **Disaster** | 81 – 100 | Severe debt, high risk of LLM hallucinations and errors |

#### 10-Dimension Scoring Model (Weights sum to 100)

1. **Setup reliability** (weight 10): Build scripts, lockfile freshness, Docker, and environment configuration.
2. **Architecture clarity** (weight 15): Directory depth, file sizes, god modules, and naming searchability.
3. **Coupling / blast radius** (weight 15): Fan-in/out, circular import cycles, and cross-package dependencies.
4. **Style consistency** (weight 10): Naming convention compliance percentage, linter and formatter adoption.
5. **Test safety net** (weight 15): Test-to-source file and line ratios, framework configuration.
6. **Runtime predictability** (weight 10): Mutable globals, import side effects, singletons, bare excepts, anti-patterns.
7. **Feedback loop speed** (weight 5): Test runner, linter configuration, and CI workflow responsiveness.
8. **Documentation** (weight 10): README quality, setup instructions, architecture docs, and contributing guides.
9. **Dependency boundaries** (weight 5): Lockfiles, `.gitignore` hygiene, and vendor/generated code separation.
10. **Context pressure** (weight 5): Token consumption relative to context budget, active surface, and dead code.

### `files`

Show per-file metrics table.

```bash
cytoscnpy files [OPTIONS] <PATH>
```

- `-j`, `--json`: Output only JSON.
- `--exclude-folders <DIRS>`: Exclude specific folders from analysis.

### `deps`

Analyze dependency hygiene with CytoScnPy rule categories:

- `CSP-R001`: imported package is missing from direct dependency declarations.
- `CSP-R002`: declared production dependency is unused.
- `CSP-R003`: imported package is present only as a transitive lockfile dependency.
- `CSP-R004`: production code imports a development dependency.
- `CSP-R005`: declared dependency belongs to the Python standard library.

```bash
cytoscnpy deps [OPTIONS] [PATHS]...
```

- `--json`: Output JSON.
- `--requirements <FILE>`: Path to a specific requirements file (default: auto-discover `requirements.txt` / `requirements-dev.txt`).
- `--ignore-unused <PKGS>`: Comma-separated package names to suppress from the unused report.
- `--ignore-missing <PKGS>`: Comma-separated import names to suppress from the missing report.
- `--exclude <DIRS>`: Folders to exclude from import scanning.
- `--extra-installed`: Also report packages installed in the venv but not declared.
- `--orphans`: Report orphan packages — installed, undeclared, not imported, and not required by any other installed package.
- `--include-dev-unused`: Include development dependencies in `CSP-R002` findings. By default, dev dependencies from dependency groups, optional dependency groups, or dev/test requirements exports are not reported as unused.
- `--fail-on-any`: Exit with code `1` if any dependency finding is found; also enables the extra-installed and orphan reports.
- `--fail-on-unused`: Exit with code `1` if unused dependencies are found.
- `--fail-on-missing`: Exit with code `1` if missing dependencies are found.
- `--fail-on-extra-installed`: Exit with code `1` if extra installed packages are found; also enables the extra-installed report.
- `--fail-on-orphans`: Exit with code `1` if orphan packages are found; also enables the orphan report.
- `--impact <PKG>`: Show which transitive packages would be removable if `<PKG>` were dropped (requires a lockfile).
- `--venv <PATH>`: Override the venv path (default: auto-detect `.venv`).
- `--lockfile <PATH>`: Override the lockfile path (default: auto-detect `uv.lock` / `poetry.lock`).
- `-O`, `--output-file <FILE>`: Save output to file.

> **Note:** Use the `deps` subcommand when you want dependency analysis in isolation or need the extra flags (`--extra-installed`, `--orphans`, `--impact`). To include dependency findings alongside the main scan, pass `--deps` to the default analysis command.

### `mcp-server`

Start MCP server for LLM integration.

```bash
cytoscnpy mcp-server [--root <PATH>]
```

- `--root <PATH>`: Confine MCP path-based tools to this project directory. If omitted, the server uses its launch working directory.

> Note: The `mcp-server` subcommand is handled by the `cytoscnpy-cli` binary. If you installed the Python package, `cytoscnpy mcp-server` will print an error. Use the standalone CLI build for MCP.

### `init`

Initialize CytoScnPy configuration in the current directory.

```bash
cytoscnpy init
```

This creates `.cytoscnpy.toml` (or appends `[tool.cytoscnpy]` to `pyproject.toml`) and adds `.cytoscnpy` to `.gitignore` when possible.

## Configuration File

Create `.cytoscnpy.toml` in your project root to set defaults.

```toml
[cytoscnpy]
# Core settings
confidence = 60
secrets = true
danger = true
quality = true
include_tests = false
include_ipynb = false
project_type = "library"   # "library" (default) or "application"
# Note: ipynb_cells is currently a CLI-only option

# Quality thresholds
max_complexity = 10        # Max cyclomatic complexity
max_nesting = 3            # Max nesting depth
max_args = 5               # Max function arguments
max_lines = 50             # Max function lines
min_mi = 40.0              # Min Maintainability Index

# Path filters
exclude_folders = ["build", "dist", ".venv"]
include_folders = ["src"]

# Rule suppression
ignore = ["CSP-P003"]      # Globally ignore specific rule IDs

# Clone detection
clones = false             # Enable duplicate code detection
clone_similarity = 0.8     # Similarity threshold (0.0-1.0)

# CI/CD
fail_threshold = 5.0
fail_on_secrets = true
fail_on_danger = true

# Per-file rule suppressions (glob -> rule IDs).
# Use a real table (not an inline `{ ... }`) so entries can span multiple
# lines without tripping TOML/editor syntax errors.
# Glob behavior: "*" matches a single path segment, "**" matches recursively.
[cytoscnpy.per-file-ignores]
"tests/*" = ["CSP-D701"]
"**/__init__.py" = ["CSP-L001"]

# Inline whitelist (suppress specific dead-code symbols).
# One array, one entry per line — no repeated headers needed.
whitelist = [
  { name = "my_handler" },                              # "exact" match (default)
  { name = "another_symbol", pattern = "wildcard" },     # "exact", "wildcard", or "regex"
  { name = "legacy_fn", file = "src/api/*.py" },         # optional: restrict to a file glob
]
```

Prefer `[[cytoscnpy.whitelist]]` blocks instead if you want to attach a comment
above each entry:

```toml
[[cytoscnpy.whitelist]]
name = "my_handler"
pattern = "exact"

[[cytoscnpy.whitelist]]
name = "another_symbol"
pattern = "wildcard"
```

### Advanced Configuration

#### Secret Scanning

```toml
[cytoscnpy.secrets_config]
entropy_threshold = 4.5
min_length = 16
entropy_enabled = true
scan_comments = true
skip_docstrings = false
min_score = 50
suspicious_names = ["db_password", "oauth_token"]

[[cytoscnpy.secrets_config.patterns]]
name = "Slack Token"
regex = "xox[baprs]-([0-9a-zA-Z]{10,48})"
severity = "HIGH"
rule_id = "CSP-SCUSTOM-001" # Optional
```

#### Dependency Analysis

```toml
[cytoscnpy.deps]
enabled = true                        # Run dep analysis by default (same as --deps)
ignore_unused = ["celery", "redis"]   # Suppress specific unused-dep findings
ignore_missing = ["numpy"]            # Suppress specific missing-dep findings
fail_on_unused = true                 # Fail if unused dependencies are reported
fail_on_missing = true                # Fail if missing dependencies are reported
fail_on_extra_installed = false       # Fail on undeclared installed packages
fail_on_orphans = false               # Fail on installed orphan packages

# Custom package → import name mappings (for packages where the import name
# differs from the package name and is not in the built-in mapping)
[cytoscnpy.deps.package_mapping]
"my-internal-lib" = ["mylib", "mylib_ext"]
```

#### Dangerous Code + Taint Analysis

```toml
[cytoscnpy.danger_config]
enable_taint = true
severity_threshold = "LOW"         # LOW, MEDIUM, HIGH, CRITICAL
excluded_rules = ["CSP-D101"]
custom_sources = ["mylib.get_input"]
custom_sinks = ["mylib.exec"]

# Sanitizers are scoped by vulnerability type and behavior.
[cytoscnpy.danger_config.sanitizers.ssrf]
return_value = ["validate_allowed_url"]
guard = ["is_allowed_url"]
side_effect = ["validate_url_or_raise"]

[cytoscnpy.danger_config.sanitizers.path_traversal]
return_value = ["safe_join"]

[cytoscnpy.danger_config.sanitizers.sql_injection]
return_value = ["build_parameterized_query"]

[cytoscnpy.danger_config.sanitizers.command_injection]
return_value = ["quote_shell_argument"]
```

Sanitizer names are matched explicitly. Security-sounding variable names do not
suppress findings, and each configured sanitizer affects only its vulnerability
type.

## Exit Codes

- `0`: Success, no issues found (or issues below threshold).
- `1`: Issues found exceeding configured gates (quality, security, dependency, or fail_threshold).
- `2`: Runtime error or invalid arguments.

## See Also

- [Usage Guide](usage.md)
