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

**Framework Support**: Automatically detects usage in Flask routes, Django views, FastAPI endpoints, and Pydantic models.

### 🔒 Security Analysis

Enable with `--secrets` and `--danger`.

**Secret Scanning**: Finds hardcoded secrets (API keys, tokens) using regex and entropy analysis.
**Dangerous Code**: Detects patterns known to cause vulnerabilities (SQLi, XSS, RCE, etc.).

For detailed vulnerability rules (`CSP-Dxxx`), see [Security Analysis](security.md).

### 📊 Code Quality Metrics

Enable with `--quality`.

- **Cyclomatic Complexity (CC)**: Measures code branching.
- **Maintainability Index (MI)**: 0-100 score (higher is better).
- **Halstead Metrics**: Algorithmic complexity.

### 🧩 Clone Detection

Finds copy-pasted code blocks.

```bash
cytoscnpy . --clones --clone-similarity 0.8
```

### 🛠️ Auto-Fix

Remove dead code automatically.

1. **Preview**: `cytoscnpy . --fix`
2. **Apply**: `cytoscnpy . --fix --apply`

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

```bash
cytoscnpy [PATHS]... [OPTIONS]
```

### Core Options

| Flag               | Description                                        |
| ------------------ | -------------------------------------------------- |
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
| `--quiet` (`-q`)   | Summary only, no detailed tables. |
| `--verbose` (`-v`) | Debug output.                     |

### Filtering

| Flag                     | Description                     |
| ------------------------ | ------------------------------- |
| `--exclude-folder <DIR>` | Exclude specific folders.       |
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

- `--all`: Enable all scanners (equivalent to `-s -d -q`).

---

## Links

- **PyPI**: [pypi.org/project/cytoscnpy](https://pypi.org/project/cytoscnpy/)
- **VS Code Extension**: [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=djinn09.cytoscnpy)
- **GitHub Repository**: [github.com/djinn09/CytoScnPy](https://github.com/djinn09/CytoScnPy/)
