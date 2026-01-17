# User Guide

This guide covers everything you need to know to use CytoScnPy effectively.

## 🚀 Quick Start

Run analysis on your current directory:

```bash
cytoscnpy . --secrets --danger --quality
```

---

## Features & Usage

### 💀 Dead Code Detection

CytoScnPy statically analyzes your code to find unused symbols. It detects:

- **Functions & Classes**: Definitions that are never called.
- **Methods**: Cascading detection (if a class is unused, its methods are too).
- **Imports**: Unused import statements.
- **Variables**: Local variables assigned but never read.

## 📦 Installation

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

### 🔒 Security Analysis

Enable with `--secrets` and `--danger`.

**Secret Scanning**: Finds hardcoded secrets (API keys, tokens) using regex and entropy analysis.
**Dangerous Code**: Detects patterns known to cause vulnerabilities (SQLi, XSS, RCE, etc.).

For detailed vulnerability rules (`CSP-Dxxx`), see the **[Dangerous Code Rules Index](dangerous-code.md)** or the general [Security Analysis](security.md) overview.

### 📊 Code Quality Metrics

Enable with `--quality`.

- **Cyclomatic Complexity (CC)**: Measures code branching.
- **Maintainability Index (MI)**: 0-100 score (higher is better).
- **Halstead Metrics**: Algorithmic complexity.

### 🧩 Clone Detection

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

### 🛠️ Auto-Fix

Remove dead code automatically.

1. **Preview**: `cytoscnpy . --fix`
2. **Apply**: `cytoscnpy . --fix --apply`

### 📄 HTML Reports

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

### ⚓ Pre-commit Hooks

Automate your analysis on every commit. See the [Pre-commit Guide](pre-commit.md) for installation and configuration.

---

## ⚙️ Configuration

CytoScnPy supports configuration via:

1.  **`.cytoscnpy.toml`** (Project root)
2.  **`pyproject.toml`** (Scanning under `[tool.cytoscnpy]`)

### Option 1: `.cytoscnpy.toml`

```toml
[cytoscnpy]
confidence = 60
exclude_folders = ["venv", "build", "dist"]
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
secrets = true
danger = true
quality = true

# CI/CD Gates
fail_threshold = 5.0
max_complexity = 15
min_mi = 40.0
```

---

## 📖 CLI Reference

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

| Flag               | Description                       |
| ------------------ | --------------------------------- |
| `--json`           | Output detection results as JSON. |
| `--html`           | Generate interactive HTML report. |
| `--quiet`          | Summary only, no detailed tables. |
| `--verbose` (`-v`) | Debug output.                     |

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

## 🔧 Troubleshooting

### Common Issues

**1. "Too many open files" error**

- Limit parallelization or exclude large directories (`node_modules`, `.git`).

**2. False Positives**

- Use **inline comments** to suppress findings on a specific line:

  | Comment                  | Effect                                                    |
  | ------------------------ | --------------------------------------------------------- |
  | `# pragma: no cytoscnpy` | Legacy format (suppresses all CytoScnPy findings)         |
  | `# noqa`                 | Bare noqa (suppresses all CytoScnPy findings)             |
  | `# ignore`               | Bare ignore (suppresses all CytoScnPy findings)           |
  | `# noqa: CSP`            | Specific (suppresses only CytoScnPy)                      |
  | `# noqa: E501, CSP`      | Mixed (suppresses CytoScnPy because `CSP` is in the list) |

  **Examples:**

  ```python
  def unused_func():  # noqa
      pass

  x = value  # noqa: CSP, E501 -- suppress cytoscnpy and pycodestyle
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
