# User Guide

This guide covers everything you need to know to use CytoScnPy effectively.

## Quick Start

Run analysis on your current directory:

```bash
cytoscnpy . --secrets --danger --quality
```

---

## Features & Usage

### Dead Code Detection

CytoScnPy statically analyzes your code to find unused symbols. It detects:

- **Functions & Classes**: Definitions that are never called.
- **Methods**: Cascading detection (if a class is unused, its methods are too).
- **Imports**: Unused import statements.
- **Variables**: Local variables assigned but never read.

## Installation

**Linux / macOS:**

```bash
# Install
curl -fsSLO https://raw.githubusercontent.com/djinn-soul/CytoScnPy/main/install.sh
bash install.sh
```

**Windows (PowerShell):**

```powershell
# Install
Invoke-WebRequest https://raw.githubusercontent.com/djinn-soul/CytoScnPy/main/install.ps1 -OutFile install.ps1
& .\install.ps1
```

**Framework Support**: Automatically detects usage in Flask routes, Django views, FastAPI endpoints, and Pydantic models.

### Security Analysis

Enable with `--secrets` and `--danger`.

**Secret Scanning**: Finds hardcoded secrets (API keys, tokens) using regex and entropy analysis.
**Dangerous Code**: Detects patterns known to cause vulnerabilities (SQLi, XSS, RCE, etc.).

For detailed vulnerability rules (`CSP-Dxxx`), see the **[Dangerous Code Rules Index](dangerous-code.md)** or the general [Security Analysis](security.md) overview.

### Code Quality Metrics

Enable with `--quality`.

- **Cyclomatic Complexity (CC)**: Measures code branching.
- **Maintainability Index (MI)**: 0-100 score (higher is better).
- **Halstead Metrics**: Algorithmic complexity.

For a full list of quality rules and their CytoScnPy rule IDs (`CSP-L001`, `CSP-Q301`, etc.), see the **[Code Quality Rules](quality.md)** reference.
Per-rule pages are available across Best Practices, Maintainability, and Performance categories.

### Rule Index

| Area                      | Reference                          |
| ------------------------- | ---------------------------------- |
| Security rules (dangerous code) | [Dangerous Code Rules](dangerous-code.md) |
| Quality rules             | [Code Quality Rules](quality.md)  |

### Dependency Analysis

CytoScnPy detects unused and missing dependencies by cross-referencing your declared dependencies (`pyproject.toml` / `requirements.txt`) against the imports actually used in your code.

**Opt-in flag** — add `--deps` to the default scan to include dependency findings alongside dead code and security:

```bash
cytoscnpy . --deps
cytoscnpy . --deps --secrets --danger
```

**Dedicated subcommand** — for dependency analysis in isolation, or to use extra flags (`--extra-installed`, `--orphans`, `--impact`):

```bash
cytoscnpy deps .
cytoscnpy deps . --extra-installed --orphans
cytoscnpy deps . --ignore-unused celery,redis
cytoscnpy deps . --impact httpx   # show what transitive deps would go with httpx
```

**CI gates** — fail a build on dependency findings. Main-scan gates enable dependency analysis automatically:

```bash
cytoscnpy . --fail-on-missing-deps --fail-on-unused-deps
cytoscnpy deps . --fail-on-missing --fail-on-unused
cytoscnpy deps . --fail-on-extra-installed --fail-on-orphans
```

**What it reports:**

| Finding | Meaning |
|---|---|
| Unused dependency | Declared in `pyproject.toml`/`requirements.txt` but never imported |
| Missing dependency | Imported in code but not declared |
| Extra installed | In the venv but not declared (transitive deps) |
| Orphan | Installed, undeclared, not imported, not needed by any other package |

**Package name aliasing** is handled automatically for common packages (e.g. `Pillow` → `PIL`, `scikit-learn` → `sklearn`, `python-dateutil` → `dateutil`). For custom internal packages, add a mapping in `.cytoscnpy.toml`. Dependency gates can also live in config:

```toml
[cytoscnpy.deps]
enabled = true
fail_on_unused = true
fail_on_missing = true
fail_on_extra_installed = false
fail_on_orphans = false

[cytoscnpy.deps.package_mapping]
"my-internal-lib" = ["mylib", "mylib_ext"]
```

**`src/` layout** is fully supported — `from src.myapp.models import Foo` will not trigger a false-positive missing report for `src`.

### Clone Detection

Finds duplicate or near-duplicate code blocks (Type-1, Type-2, and Type-3 clones).

```bash
cytoscnpy . --clones --clone-similarity 0.8
```

- **Type-1**: Exact copies (identical code).
- **Type-2**: Syntactically identical (variable renaming).
- **Type-3**: Near-miss clones (small edits/additions).

**Options:**

- `--clone-similarity <0.0-1.0>`: Minimum similarity threshold. Default is `0.8` (80% similarity). Lower values find more matches but may increase false positives.

**Performance**: Clone detection is computationally intensive for very large codebases.

### Auto-Fix

Remove dead code automatically.

1. **Preview**: `cytoscnpy . --fix`
2. **Apply**: `cytoscnpy . --fix --apply`

Generate and reuse a whitelist to suppress intended dead-code items:

```bash
cytoscnpy . --make-whitelist > whitelist.py
cytoscnpy . --whitelist whitelist.py
```

### HTML Reports

Generate interactive, self-contained HTML reports for easier navigation of findings.

```bash
cytoscnpy . --html --secrets --danger
```

(Note: `--html` automatically enables `--quality` but strictly security scans need explicit flags).

**Features:**

- **Dashboard**: High-level summary of issues.
- **Search**: Interactive search across all findings.
- **Filtering**: Filter by severity, category, or file.
- **Source View**: Clickable file links with line numbers.

**When to use HTML vs JSON:**

- Use **HTML** for human review and team sharing.
- Use **JSON** (`--json`) for CI/CD pipelines and automated processing.

---

---

## CI/CD Integration

CytoScnPy is designed to work seamlessly with modern CI/CD pipelines. Using the `--root` flag and specific `--format` options, you can integrate analysis results directly into your build process.

> [!IMPORTANT]
> Always use `--root .` (or your project path) in CI/CD. This ensures that:
>
> 1. Absolute paths are correctly normalized to relative paths in reports.
> 2. Security containment boundaries are correctly established.
> 3. Fingerprints (for GitLab/GitHub) remain stable across different build runners.

### GitLab Code Quality

Generate a report that GitLab can display directly in Merge Requests.

```yaml
# .gitlab-ci.yml
code_quality:
  stage: test
  image: python:3.9
  script:
    - pip install cytoscnpy
    - cytoscnpy --root . --format gitlab --danger --secrets > gl-code-quality-report.json
  artifacts:
    reports:
      codequality: gl-code-quality-report.json
```

### GitHub Actions

Generate inline annotations for your Pull Requests.

```yaml
# .github/workflows/scan.yml
jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install CytoScnPy
        run: pip install cytoscnpy
      - name: Run Scan
        run: cytoscnpy --root . --format github --danger --secrets
```

### SARIF (GitHub Security / GitLab Security)

Export results in the standard Static Analysis Results Interchange Format (SARIF).

```bash
cytoscnpy --root . --format sarif --danger > results.sarif
```

### JUnit XML

Integration with test runners and CI platforms that support JUnit (Azure DevOps, Jenkins).

```bash
cytoscnpy --root . --format junit --quality > test-results.xml
```

---

### ⚓ Pre-commit Hooks

---

## ⚙️ Configuration

CytoScnPy supports configuration via:

1.  **`.cytoscnpy.toml`** (Project root)
2.  **`pyproject.toml`** (Scanning under `[tool.cytoscnpy]`)

You can scaffold a default config with:

```bash
cytoscnpy init
```

### Option 1: `.cytoscnpy.toml`

```toml
[cytoscnpy]
confidence = 60
exclude_folders = ["venv", "build", "dist"]
include_folders = ["src"]
include_tests = false
secrets = true
danger = true
quality = true
include_ipynb = false
project_type = "library"   # "library" (default) or "application"
ignore = ["CSP-P003"]      # Global rule suppressions

# Clone detection
clones = false             # Enable duplicate code detection
clone_similarity = 0.8     # Similarity threshold (0.0-1.0)

# CI/CD Gates (Fail if exceeded)
fail_threshold = 5.0   # >5% unused code
max_complexity = 15    # Function CC > 15
min_mi = 40.0          # MI < 40
fail_on_secrets = true # Any secret finding
fail_on_danger = true  # Any danger or taint finding

# Per-file rule suppressions (glob -> rule IDs). Must be its own table
# (not an inline `{ ... }`) so entries can span multiple lines without
# TOML syntax errors.
[cytoscnpy.per-file-ignores]
"tests/*" = ["CSP-D701"]
"**/__init__.py" = ["CSP-L001"]
```

### Option 2: `pyproject.toml`

```toml
[tool.cytoscnpy]
confidence = 60
exclude_folders = ["venv", "build", "dist"]
include_folders = ["src"]
include_tests = false
secrets = true
danger = true
quality = true
project_type = "library"
ignore = ["CSP-P003"]

# Clone detection
clones = false
clone_similarity = 0.8

# CI/CD Gates
fail_threshold = 5.0
max_complexity = 15
min_mi = 40.0
fail_on_secrets = true
fail_on_danger = true

# Per-file rule suppressions (glob -> rule IDs). Must be its own table
# (not an inline `{ ... }`) so entries can span multiple lines without
# TOML syntax errors.
[tool.cytoscnpy.per-file-ignores]
"tests/*" = ["CSP-D701"]
"**/__init__.py" = ["CSP-L001"]
```

### Advanced Config (Security)

Use nested tables to customize secret scanning and taint analysis:

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

[cytoscnpy.danger_config]
enable_taint = true
severity_threshold = "LOW"          # LOW, MEDIUM, HIGH, CRITICAL
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

Use `return_value` for functions that return a safe value, `guard` for predicates
that establish safety only inside their truthy branch, and `side_effect` for
validators that return normally only when the argument is safe.

`project_type` controls dead-code export assumptions:

- `library` (default): public symbols are treated as potential API.
- `application`: reduces public-API assumptions for app-style codebases.

---

## CLI Reference

For a complete reference, see [docs/CLI.md](CLI.md).

```bash
cytoscnpy [PATHS]... [OPTIONS]
```

### Core Options

| Flag               | Description                                        |
| ------------------ | -------------------------------------------------- |
| `--root <PATH>`    | Project root for analysis (CI/CD mode).            |
| `--confidence <N>` (`-c`) | Minimum confidence threshold (0-100). Default: 60. |
| `--secrets` (`-s`) | Scan for API keys, tokens, credentials.            |
| `--danger` (`-d`)  | Scan for dangerous code + taint analysis.          |
| `--quality` (`-q`) | Scan for code quality issues.                      |
| `--clones`         | Enable code clone detection.                       |
| `--clone-similarity <N>` | Set clone similarity threshold (0.0-1.0, default `0.8`). |
| `--no-dead` (`-n`) | Skip dead code detection.                          |
| `--deps`           | Analyze unused/missing dependencies (opt-in).      |
| `--make-whitelist` | Generate whitelist entries from current findings.  |
| `--whitelist <PATH>` | Load whitelist file(s) for dead-code suppression. |

### Output Formatting

| Flag               | Description                                                                      |
| ------------------ | -------------------------------------------------------------------------------- |
| `--format <FMT>`   | Output format: `text`, `json`, `junit`, `github`, `gitlab`, `markdown`, `sarif`. |
| `--json`           | Output detection results as JSON (shorthand for `--format json`).                |
| `--html`           | Generate interactive HTML report.                                                |
| `--quiet`          | Summary only, no detailed tables.                                                |
| `--verbose` (`-v`) | Debug output.                                                                    |

### Filtering

| Flag                     | Description                     |
| ------------------------ | ------------------------------- |
| `--exclude-folders <DIRS>` | Exclude specific folders.       |
| `--include-folders <DIRS>` | Force include folders.          |
| `--include-tests`        | Include test files in analysis, metrics, statistics, and clone detection. |
| `--include-ipynb`        | Include Jupyter notebooks.      |

### CI/CD Failure Gates

CytoScnPy can enforce quality, security, and dependency standards by exiting with code `1`. Security and dependency failure gates enable their required scans automatically.

| Flag                   | Description                          |
| ---------------------- | ------------------------------------ |
| `--fail-threshold <N>` | Fail if unused code % > N.           |
| `--max-complexity <N>` | Fail if any function complexity > N. |
| `--min-mi <N>`         | Fail if Maintainability Index < N.   |
| `--fail-on-quality`    | Fail on any quality issue.           |
| `--fail-on-secrets`    | Fail on any secret finding.          |
| `--fail-on-danger`     | Fail on any danger or taint finding. |
| `--fail-on-missing-deps` | Fail on any missing dependency.     |
| `--fail-on-unused-deps` | Fail on any unused dependency.      |

### Subcommands

CytoScnPy has specialized subcommands for specific metrics.

#### `hal` - Halstead Metrics

```bash
cytoscnpy hal . --functions
```

Calculates Halstead complexity metrics.

- `--functions`: Compute at function level.

#### `files` - Per-File Metrics

```bash
cytoscnpy files . --json
```

Shows detailed metrics table for each file.

#### `cc` - Cyclomatic Complexity

```bash
cytoscnpy cc . --min-rank C --show-complexity
```

Calculates McCabe complexity.

- `--show-complexity`: Show score.
- `--min-rank <A-F>`: Filter by rank (A=Simple ... F=Critical).
- `--max-complexity <N>`: Fail if complexity > N.

#### `mi` - Maintainability Index

```bash
cytoscnpy mi . --show --average
```

Calculates Maintainability Index (0-100).

- `--show`: Show values.
- `--fail-threshold <N>`: Fail if MI < N.

### Additional Quality Options

| Flag                | Description                                  |
| ------------------- | -------------------------------------------- |
| `--max-nesting <N>` | Fail if nesting depth > N.                   |
| `--max-args <N>`    | Fail if function arguments > N.              |
| `--max-lines <N>`   | Fail if function lines > N.                  |
| `--ipynb-cells`     | Report findings at cell level for notebooks. |

#### `raw` - Raw Metrics

```bash
cytoscnpy raw . --json
```

Calculates LOC, SLOC, Comments, Blank lines.

#### `stats` - Project Statistics

```bash
cytoscnpy stats . --all
```

Runs full analysis (secrets, danger, quality) and prints summary statistics.

**Options:**

- `--all` (`-a`): Enable all scanners (secrets, danger, quality).
- `--secrets` (`-s`): Enable secret scanning.
- `--danger` (`-d`): Enable danger/taint analysis.
- `--quality` (`-q`): Enable quality analysis.
- `--exclude-folders <DIRS>`: Exclude specific folders from stats analysis.
- `--json`: Output as JSON.
- `--output <FILE>` (`-o`): Save report to file.

#### `deps` - Dependency Analysis

```bash
cytoscnpy deps . --extra-installed --orphans
```

Analyzes unused and missing dependencies in isolation.

- `--extra-installed`: Show packages installed but not declared.
- `--orphans`: Show zombie packages (installed, undeclared, not imported).
- `--impact <PKG>`: Show transitive packages removable with `<PKG>`.
- `--ignore-unused <PKGS>`: Suppress specific unused-dep findings.
- `--ignore-missing <PKGS>`: Suppress specific missing-dep findings.
- `--fail-on-unused`: Fail if unused dependencies are found.
- `--fail-on-missing`: Fail if missing dependencies are found.
- `--fail-on-extra-installed`: Fail if extra installed packages are found.
- `--fail-on-orphans`: Fail if orphan packages are found.
- `--venv <PATH>`: Override venv path (default: auto-detect `.venv`).

#### `mcp-server` - MCP Integration

```bash
cytoscnpy mcp-server
```

Starts the Model Context Protocol (MCP) server for integration with AI assistants (Claude Desktop, Cursor, GitHub Copilot).

---

## Troubleshooting

### Common Issues

**1. "Too many open files" error**

- Limit parallelization or exclude large directories (`node_modules`, `.git`).

**2. False Positives**

- Use **inline comments** to suppress findings on a specific line:

  | Comment                  | Effect                                            |
  | ------------------------ | ------------------------------------------------- |
  | `# pragma: no cytoscnpy` | Legacy format (suppresses all CytoScnPy findings) |
  | `# noqa`                 | Bare noqa (suppresses all CytoScnPy findings)                |
  | `# ignore`               | Bare ignore (suppresses all CytoScnPy findings)              |
  | `# noqa: CSP-XXXX`       | Specific rule suppression (danger/quality/performance rules) |

  **Examples:**

  ```python
  def mutable_default(arg=[]):  # noqa
      pass

  x = [1, 2] == None # noqa: CSP-L003
  for x in items:  # noqa: CSP-P003
      out += x
  y = api_key  # pragma: no cytoscnpy
  ```

- For bulk ignores, use the `.cytoscnpy.toml` configuration file's ignore list.
  For per-file rule scopes, add a `[tool.cytoscnpy.per-file-ignores]` table (or `per-file-ignores` inside `.cytoscnpy.toml`) with glob keys mapped to rule lists:

  ```toml
  [tool.cytoscnpy.per-file-ignores]
  "tests/*" = ["CSP-D701"]       # Allow assert statements in tests
  "**/__init__.py" = ["CSP-L001"] # Example targeted suppression
  "migrations/**" = ["CSP-P003"]  # Example recursive glob
  ```

  Glob behavior matches Ruff-style semantics:
  - `*` matches within a single directory level.
  - `**` matches recursively across directories.

**3. Performance is slow**

- Check if large files or build artifacts are being scanned. Use `--exclude-folders`.
- Clone detection is slower than standard analysis.

**4. "Missing integrity" finding**

- Security check requires SRI hashes for external scripts. Add `integrity="..."` to your HTML.

---

## Links

- **PyPI**: [pypi.org/project/cytoscnpy](https://pypi.org/project/cytoscnpy/)
- **VS Code Extension**: [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=djinn09.cytoscnpy)
- **GitHub Repository**: [github.com/djinn-soul/CytoScnPy](https://github.com/djinn-soul/CytoScnPy/)
