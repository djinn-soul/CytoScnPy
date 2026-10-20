# DeSlop: LLM Friction Score

`cytoscnpy deslop` measures how hard a Python codebase is for an LLM agent (or a
new contributor) to navigate and change safely. It runs one pass over the
project and reports a **Slop Index** from 0 to 100, the evidence behind it, a
ranked list of fixes, and pass/fail gates for CI.

It is pure static analysis: no LLM calls, no network, no tokens spent.

```bash
cytoscnpy deslop .                      # Slop Index, dimensions, top fixes, gate status
cytoscnpy deslop . --verbose            # Every analysis section with file locations
cytoscnpy deslop . --json -o slop.json  # Full structured report
cytoscnpy deslop . --format llm         # Fix plan written for an LLM agent
cytoscnpy deslop . --ci                 # Fail when the Slop Index is above 40
cytoscnpy deslop . --fail-on-any        # Fail on any configured gate
```

## What It Analyzes

One `deslop` run covers all of these; none needs a separate command.

| Area | What it looks at | JSON field |
|---|---|---|
| Architecture | Import graph, fan-in/fan-out, circular imports, god modules, layering | `architecture` |
| Git context | Active vs. frozen files, churn, high-churn + high-complexity hotspots, token budget | `context` |
| Repository health | Dependency manager, lockfile, Docker, `.gitignore`, README, CI, tooling | `health` |
| Searchability | Duplicate filenames, colliding and generic function names | `searchability` |
| Naming | Naming-style distribution and consistency | `naming` |
| TODOs | TODO/FIXME/HACK/XXX markers and debug prints | `todos` |
| Globals | Mutable module-level state | `globals` |
| Exceptions | Bare `except:` and empty handlers | `exceptions` |
| Wildcard imports | `from x import *` | `wildcards` |
| Side effects | Work done at import time | `side_effects` |
| Singletons | Singleton patterns and mutable class variables | `singletons` |
| Anti-patterns | Magic numbers, deeply nested callbacks, and similar | `anti_patterns` |
| Duplicates | Duplicate-code clusters and duplicated lines | `duplicates` |
| Unreferenced code | Large functions in files nothing imports | `unreferenced` |
| Functions | Length, cyclomatic complexity, and nesting per function | `functions` |

Dead code, dependency, secret, danger, and quality-rule scans stay on their own
commands (`cytoscnpy .`, `cytoscnpy deps`, and so on). See the
[CLI Reference](CLI.md).

### What Gets Scanned

- With no path, the current directory is scanned. You can pass several files
  and directories, or use `--root <PATH>` instead of positional paths.
- Only Python files are analyzed. Other files still count toward the codebase
  summary and language breakdown.
- `.gitignore`, `.git/info/exclude`, and your global Git ignore file are
  respected.
- Common non-source folders are always skipped: virtual environments (`venv`,
  `.venv`, `env`, `.env`), caches (`__pycache__`, `.pytest_cache`,
  `.mypy_cache`, `.ruff_cache`, `.tox`, `.nox`), build output (`build`, `dist`,
  `*.egg-info`, `target`), and vendored or generated code (`site-packages`,
  `node_modules`, `vendor`, `third_party`, `generated`).
- Tests are skipped unless you pass the global `--include-tests` flag
  (`cytoscnpy --include-tests deslop .`) or set `include_tests = true` in the
  config.
- Sample-code directories (`examples/`, `fixtures/`, `test_fixtures/`,
  `testdata/`, `test_data/`) below the analysis root are skipped together with
  tests, because they often hold deliberately sloppy code. `--include-tests`
  scans them again, and pointing `deslop` directly at one still analyzes it.
- Test files are classified relative to the analysis root, so a `tests/`
  directory above the project does not mark every file as a test.
- `--exclude` and `--ignore` remove further paths.

## How the Slop Index Is Computed

### 1. Ten Weighted Dimensions

Each dimension is rated from **0** (no friction) to **5** (maximum friction).
Its contribution is `weight × rating / 5`, and the weights sum to 100.

| # | Dimension | Weight | What raises the rating |
|---|---|---|---|
| 1 | Setup reliability | 10 | One point for each missing dependency manager, lockfile, Docker config, or `.gitignore`. A Makefile or other task runner is optional. |
| 2 | Architecture clarity | 15 | Deep directories, large Python files, god modules, poor searchability |
| 3 | Coupling / blast radius | 15 | High fan-in/fan-out, circular imports, cross-package dependencies |
| 4 | Style consistency | 10 | No linter, formatter, or type checker; mixed naming styles |
| 5 | Test safety net | 15 | Low test-to-source ratio, no test framework config |
| 6 | Runtime predictability | 10 | Mutable globals, import side effects, singletons, bare excepts, anti-patterns |
| 7 | Feedback loop speed | 5 | Slow estimated feedback, no test runner, no CI |
| 8 | Documentation | 10 | Missing README, setup steps, architecture docs, contributing guide |
| 9 | Dependency boundaries | 5 | No lockfile or `.gitignore`, vendored or generated code mixed with source |
| 10 | Context pressure | 5 | Large token volume, long or deeply nested functions, dead code |

The test ratio compares Python test lines with Python source lines only.
Markdown, JSON, and other non-code files do not count as source.

The sum of contributions is the **raw score** (0–100).

### 2. Size Multiplier

A small codebase is easy to hold in context even when it is messy, so the raw
score is scaled by how much of the context window the code would fill:

```text
tokens          = effective bytes / 3.5
size multiplier = min(1.0, (tokens / context budget) ^ 1.5)
Slop Index      = raw score × size multiplier
```

- **Effective bytes**: when Git history is available, only the *active* files
  (changed within the lookback window) count. Stable, frozen code is read but
  rarely changed, so it adds less friction. With `--no-git`, or outside a Git
  repository, all bytes count.
- **Context budget**: usable context in tokens, `176000` by default. Change it
  with `--context-budget`.

A codebase whose active code fills the whole budget gets the full raw score.

### 3. Verdict

| Verdict | Slop Index | Meaning |
|---|---|---|
| **CLEAN** | 0 – 20 | Negligible LLM friction |
| **ACCEPTABLE** | 21 – 40 | Normal codebase, minor friction |
| **MESSY** | 41 – 60 | Moderate friction, refactoring recommended |
| **SLOPPY** | 61 – 80 | High friction, hard to navigate and maintain |
| **DISASTER** | 81 – 100 | Severe debt, high risk of agent mistakes |

If any dimension is rated 5/5, the verdict is at least **ACCEPTABLE**, even when
the size multiplier pushes the index into the CLEAN band. Only the label
changes; the index and gates do not.

## Reading the Report

```text
+-------------------------------------------------------------------------+
|                        DESLOPIFY SLOP INDEX REPORT                      |
+-------------------------------------------------------------------------+
|  Slop Index      :   2 / 100   [CLEAN - Pristine / Low LLM Friction]    |
|  Raw Score       :  32.00 / 100                                         |
|  Size Multiplier :   0.08x (effective tokens vs context window)         |
+-------------------------------------------------------------------------+

Dimension Breakdown:
  DIMENSION                  WEIGHT   RATING   RAW POINTS  EVIDENCE
  Setup reliability              10      3/5         6.00  no lockfile, no Docker config, no .gitignore
  Style consistency              10      3/5         6.00  no formatter/linter/type-checker, naming consistency: 100.0%
  ...

Top Prioritized Remediations:
  1. [-1 pts, 6.0 raw] Configure code formatter and linter (Ruff / Black) (Style consistency, Effort: Low)
     Files: pyproject.toml
  2. [-1 pts, 4.0 raw] Document architecture in docs/ARCHITECTURE.md (Documentation, Effort: Medium)
     Files: docs/ARCHITECTURE.md

[GATE PASSED] Codebase satisfies slop index and threshold requirements.
```

- The default report shows the index, codebase summary, dimension breakdown,
  up to 15 remediations, and gate status. `--verbose` adds every analysis
  section with file locations.
- Each remediation shows two numbers: the **index points** it would save after
  the size multiplier (`-1 pts`) and the **raw score** it would save before it
  (`6.0 raw`). Fixes are ranked by raw reduction, so the order stays useful on
  small codebases where every fix rounds to one index point.
- The tooling line lists up to six detected configurations; the JSON report
  has the full list.

### Verbose Report Sections

`--verbose` prints the complete dimension breakdown and health diagnostics,
then one section per analysis:

| Section | Shows |
|---|---|
| Architecture | Repository metrics, most depended-on modules (fan-in), heaviest consumers (fan-out), cycles, god modules |
| Git Context and Hotspots | Active vs. frozen code surface, most-churned files, churn × complexity hotspots, navigation token budget |
| Repository Health | Setup reliability, detected tooling, structure and language volume, setup recommendations |
| Codebase Searchability | Duplicate filenames, function name collisions, generic identifiers |
| Naming Style Consistency | Style distribution and consistency score |
| TODO / Annotation Scan | TODO/FIXME/HACK/XXX markers and debug prints |
| Mutable Global State | Module-level mutable state |
| Exception Handler Anti-Patterns | Bare `except:` and empty handlers |
| Wildcard Imports | `from x import *` |
| Module-Level Side Effects | Code that runs at import time |
| Singleton Patterns | Singletons and mutable class variables |
| Anti-Patterns | Magic numbers, nested callbacks, and similar |
| Duplicate Code Clusters | Duplicated blocks and their locations |
| Unreferenced Large Functions | Large functions in files nothing imports |
| Python Function Metrics | Longest, most complex, and most deeply nested functions |
| CI Gates | Each gate failure with its actual value and limit |

## Output Formats

| Option | Output |
|---|---|
| *(default)* or `--format terminal` | Human-readable report |
| `--json` or `--format json` | One JSON document with every analysis |
| `--format llm` | Markdown remediation plan for an LLM agent: a health snapshot, then numbered steps with target files and implementation steps |

An explicit `--format` takes precedence over `--json`. Use `-o <FILE>` to write
the report to a file.

### JSON Report

Top-level fields:

| Field | Contents |
|---|---|
| `schema_version` | Report schema version (currently `1`) |
| `slop_index`, `verdict`, `raw_score`, `size_multiplier` | Headline score |
| `dimensions` | `name`, `weight`, `rating`, `raw_contribution`, `evidence` per dimension |
| `recommendations` | `id`, `title`, `dimension`, `estimated_reduction`, `raw_reduction`, `target_rating`, `effort`, `description`, `action_steps`, `affected_files` |
| `scoring` | Full scoring result, including `passed_gate` and `failure_reason` |
| `architecture` … `functions` | One field per analysis (see [What It Analyzes](#what-it-analyzes)) |
| `scan_integrity` | `files_discovered`, `files_checked`, `complete`, and `issues` (`path`, `reason`) |
| `gates` | `passed` and `failures` (`check`, `actual`, `limit`) |

```bash
cytoscnpy deslop . --json | jq '{slop_index, verdict, gates}'
```

## Gates and Exit Codes

`deslop` exits with code `1` when any gate fails, and `0` otherwise.

| Gate | When it runs |
|---|---|
| `scan_integrity` | Always. Fails when a file cannot be discovered, read, or parsed, so CI never treats a partial scan as a pass. |
| `no_python_files` | Always. Fails when the scan finds no Python files, instead of reporting a clean score for an empty or wrong path. |
| `slop_index` | When a ceiling is set: `--max-score <N>`, or `max_slop_index` from the config with `--ci` or `--fail-on-any`, or `40` with `--ci` alone. |
| Configured limits | Only with `--fail-on-any`. See [Configuration](#configuration). |

`--fail-on-any` also works in its global position:
`cytoscnpy --fail-on-any deslop .`.

## Configuration

Set gate limits under `[cytoscnpy.deslop]` in `.cytoscnpy.toml`, or
`[tool.cytoscnpy.deslop]` in `pyproject.toml`. The first four keys are always
checked with `--fail-on-any`; the rest are checked only when set.

| Key | Default | Fails when |
|---|---|---|
| `max_cycles` | `0` | Circular import components exceed the limit |
| `max_god_modules` | `0` | God modules exceed the limit |
| `max_hotspots` | `0` | High or critical churn hotspots exceed the limit |
| `min_health_score` | `70` | A project root's setup score is below the limit |
| `max_navigation_pct` | unset | Navigation uses more than this % of the context budget |
| `max_duplicate_filenames` | unset | Duplicate filenames exceed the limit |
| `max_function_collisions` | unset | Colliding function names exceed the limit |
| `min_naming_consistency` | unset | Naming consistency is below the ratio (`0.85` or `85`) |
| `max_todos` | unset | TODO/FIXME/HACK/XXX markers and debug prints exceed the limit |
| `max_global_mutables` | unset | Mutable globals exceed the limit |
| `max_bare_excepts` | unset | Bare `except:` blocks exceed the limit |
| `max_empty_handlers` | unset | Empty exception handlers exceed the limit |
| `max_wildcard_imports` | unset | `from x import *` imports exceed the limit |
| `max_side_effects` | unset | Module-level side effects exceed the limit |
| `max_singletons` | unset | Singleton patterns exceed the limit |
| `max_anti_patterns` | unset | Anti-patterns exceed the limit |
| `max_magic_numbers` | unset | Magic numbers exceed the limit |
| `max_nested_callbacks` | unset | Deeply nested callbacks exceed the limit |
| `max_duplicate_clusters` | unset | Duplicate-code clusters exceed the limit |
| `max_duplicate_lines` | unset | Duplicate lines exceed the limit |
| `max_duplicate_pct` | unset | Duplicate code exceeds this percentage |
| `max_unreferenced_functions` | unset | Unreferenced large functions exceed the limit |
| `max_unreferenced_lines` | unset | Lines in unreferenced large functions exceed the limit |
| `max_function_lines` | unset | Any function is longer than the limit |
| `max_function_complexity` | unset | Any function's cyclomatic complexity exceeds the limit |
| `max_nesting_depth` | unset | Any function's nesting depth exceeds the limit |
| `max_slop_index` | unset | The Slop Index exceeds the limit (with `--ci` or `--fail-on-any`) |

```toml
[tool.cytoscnpy.deslop]
max_cycles = 0
max_god_modules = 0
max_hotspots = 0
min_health_score = 70
max_slop_index = 40
max_function_complexity = 15
max_duplicate_pct = 5.0
# min_naming_consistency = 0.85
# max_navigation_pct = 75.0
```

Unknown keys are rejected, so a typo stops config loading with an error instead
of silently disabling a gate.

## CI Integration

=== "GitHub Actions"

    ```yaml
    - name: Slop Index gate
      run: |
        pip install cytoscnpy
        cytoscnpy deslop . --fail-on-any --json -o deslop.json

    - name: Upload report
      if: always()
      uses: actions/upload-artifact@v4
      with:
        name: deslop-report
        path: deslop.json
    ```

=== "GitLab CI"

    ```yaml
    deslop:
      script:
        - pip install cytoscnpy
        - cytoscnpy deslop . --fail-on-any --json -o deslop.json
      artifacts:
        when: always
        paths: [deslop.json]
    ```

Start with `--ci` (index ceiling only), then add limits to `[cytoscnpy.deslop]`
and switch to `--fail-on-any` as the codebase improves.

## Options

| Option | Description |
|---|---|
| `<PATH>...` | Files or directories to analyze (default: current directory) |
| `--root <PATH>` | Analyze this directory and use it as the path-containment boundary; cannot be combined with positional paths |
| `--fail-on-any` | Fail when any configured gate fails |
| `--ci` | Apply the Slop Index ceiling (`40` unless `--max-score` or `max_slop_index` sets one) |
| `--max-score <N>` | Maximum allowed Slop Index (0–100) |
| `--json` | Output one JSON report |
| `--format <terminal\|json\|llm>` | Choose the output; overrides `--json` |
| `-v`, `--verbose` | Show every analysis section and full scoring diagnostics |
| `-o`, `--output <FILE>` | Write the report to a file |
| `--exclude <DIR>` | Exclude a folder or path pattern (alias `--exclude-folder`) |
| `-i`, `--ignore <PATTERN>` | Ignore a path or directory pattern (repeatable) |
| `--no-git` | Skip Git history; score against all bytes |
| `--git-months <N>` | Maximum Git lookback in months (1 or more) |
| `--context-budget <TOKENS>` | Usable LLM context in tokens (default `176000`) |

Global flags go **before** `deslop`:

| Flag | Effect on `deslop` |
|---|---|
| `--include-tests` | Also scan tests and sample-code directories |
| `--fail-on-any` | Same as the subcommand flag |

```bash
cytoscnpy --include-tests deslop . --json
```

## Troubleshooting

**The raw score is high but the Slop Index is low.**
The size multiplier scales the score down for codebases much smaller than the
context budget. The ranked remediations still apply. Lower `--context-budget`
to model an agent with less context, or add `--no-git` to count frozen code too.

**`scan_integrity` fails.**
At least one file could not be read or parsed. `scan_integrity.issues` (or the
CI Gates section with `--verbose`) lists each path and the reason. Fix the
syntax error, or exclude the file with `--exclude` or `--ignore`.

**`no_python_files` fails.**
The path has no Python files after excludes, or every file is a test or
sample file. Check the path, or add `--include-tests` before `deslop`.

**`--include-tests` is rejected.**
It is a global flag: write `cytoscnpy --include-tests deslop .`, not
`cytoscnpy deslop . --include-tests`.

**A config key has no effect.**
Most limits are checked only with `--fail-on-any`, and `max_slop_index` only
with `--ci` or `--fail-on-any`. A misspelled key is an error, not ignored.

**Hotspot and churn data are empty.**
The scan target is not in a Git repository, `--no-git` is set, or no files
changed within the lookback window. Increase `--git-months` to widen it.
