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
curl -fsSL https://raw.githubusercontent.com/djinn09/CytoScnPy/main/install.sh | bash
```

**Windows (PowerShell):**

```powershell
# Install
irm https://raw.githubusercontent.com/djinn09/CytoScnPy/main/install.ps1 | iex
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

For a full list of quality rules and their standard IDs (B006, E722, etc.), see the **[Code Quality Rules](quality.md)** reference.
Note: Per-rule pages exist only for Best Practices and Performance; maintainability rules are summarized in the index.

### Rule Index

| Area                      | Reference                          |
| ------------------------- | ---------------------------------- |
| Security rules (dangerous code) | [Dangerous Code Rules](dangerous-code.md) |
| Quality rules             | [Code Quality Rules](quality.md)  |

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

# CI/CD Gates (Fail if exceeded)
fail_threshold = 5.0   # >5% unused code
max_complexity = 15    # Function CC > 15
min_mi = 40.0         # MI < 40
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

# CI/CD Gates
fail_threshold = 5.0
max_complexity = 15
min_mi = 40.0
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
severity_threshold = "LOW" # LOW, MEDIUM, HIGH, CRITICAL
excluded_rules = ["CSP-D101"]
custom_sources = ["mylib.get_input"]
custom_sinks = ["mylib.exec"]
```

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
| `--confidence <N>` | Minimum confidence threshold (0-100). Default: 60. |
| `--secrets` (`-s`) | Scan for API keys, tokens, credentials.            |
| `--danger` (`-d`)  | Scan for dangerous code + taint analysis.          |
| `--quality` (`-q`) | Scan for code quality issues.                      |
| `--clones`         | Enable code clone detection.                       |
| `--no-dead` (`-n`) | Skip dead code detection.                          |

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
| `--exclude-folder <DIR>` | Exclude specific folders.       |
| `--include-folder <DIR>` | Force include folders.          |
| `--include-tests`        | Include test files in analysis. |
| `--include-ipynb`        | Include Jupyter notebooks.      |

### CI/CD Quality Gates

CytoScnPy can enforce quality standards by exiting with code `1`:

| Flag                   | Description                          |
| ---------------------- | ------------------------------------ |
| `--fail-threshold <N>` | Fail if unused code % > N.           |
| `--max-complexity <N>` | Fail if any function complexity > N. |
| `--min-mi <N>`         | Fail if Maintainability Index < N.   |
| `--fail-on-quality`    | Fail on any quality issue.           |

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
- `--exclude-folder <DIR>`: Exclude specific folders from stats analysis.
- `--json`: Output as JSON.
- `--output <FILE>` (`-o`): Save report to file.

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

**3. Performance is slow**

- Check if large files or build artifacts are being scanned. Use `--exclude-folder`.
- Clone detection is slower than standard analysis.

**4. "Missing integrity" finding**

- Security check requires SRI hashes for external scripts. Add `integrity="..."` to your HTML.

---

## Links

- **PyPI**: [pypi.org/project/cytoscnpy](https://pypi.org/project/cytoscnpy/)
- **VS Code Extension**: [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=djinn09.cytoscnpy)
- **GitHub Repository**: [github.com/djinn09/CytoScnPy](https://github.com/djinn09/CytoScnPy/)
