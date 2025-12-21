# CytoScnPy CLI Reference

Complete command-line reference for CytoScnPy.

## Main Command

```bash
cytoscnpy [PATHS]... [OPTIONS]
```

### Arguments

| Argument | Description                                                  |
| -------- | ------------------------------------------------------------ |
| `PATHS`  | Files or directories to analyze (default: current directory) |

### Core Options

| Flag               | Short | Description                              |
| ------------------ | ----- | ---------------------------------------- |
| `--confidence <N>` | `-c`  | Confidence threshold 0-100 (default: 60) |
| `--secrets`        |       | Scan for API keys, tokens, credentials   |
| `--danger`         |       | Scan for dangerous code + taint analysis |
| `--quality`        |       | Scan for code quality issues             |

### Output Options

| Flag        | Short | Description                                  |
| ----------- | ----- | -------------------------------------------- |
| `--json`    |       | Output results as JSON                       |
| `--verbose` | `-v`  | Enable verbose output (debug mode)           |
| `--quiet`   | `-q`  | Quiet mode: summary only, no detailed tables |

### Include/Exclude Options

| Flag                     | Description                                |
| ------------------------ | ------------------------------------------ |
| `--include-tests`        | Include test files in analysis             |
| `--include-ipynb`        | Include Jupyter notebooks (.ipynb)         |
| `--ipynb-cells`          | Report findings per notebook cell          |
| `--exclude-folder <DIR>` | Exclude specific folders                   |
| `--include-folder <DIR>` | Force include folders (overrides defaults) |

### CI/CD Gate Options

| Flag                   | Description                                |
| ---------------------- | ------------------------------------------ |
| `--fail-threshold <N>` | Exit code 1 if unused code % > N           |
| `--max-complexity <N>` | Exit code 1 if any function complexity > N |
| `--min-mi <N>`         | Exit code 1 if maintainability index < N   |
| `--fail-on-quality`    | Exit code 1 if any quality issues found    |

**Priority:** CLI flag > config file > environment variable > default

**Environment Variable:** `CYTOSCNPY_FAIL_THRESHOLD=5.0`

### MCP Server

Start an MCP (Model Context Protocol) server over stdio for AI assistant integration:

```bash
cytoscnpy mcp-server
```

This enables AI assistants (Claude, GitHub Copilot, Cursor) to use CytoScnPy's analysis tools. See [../cytoscnpy-mcp/README.md](../cytoscnpy-mcp/README.md) for configuration.

---

## Metric Subcommands

### `raw` - Raw Metrics

Calculate LOC, LLOC, SLOC, Comments, Multi, Blank.

```bash
cytoscnpy raw [PATH] [OPTIONS]
```

| Flag            | Short | Description                      |
| --------------- | ----- | -------------------------------- |
| `--json`        | `-j`  | Output JSON                      |
| `--exclude`     | `-e`  | Exclude folders                  |
| `--ignore`      | `-i`  | Ignore directories matching glob |
| `--summary`     | `-s`  | Show summary of gathered metrics |
| `--output-file` | `-O`  | Save output to file              |

---

### `cc` - Cyclomatic Complexity

Calculate McCabe cyclomatic complexity.

```bash
cytoscnpy cc [PATH] [OPTIONS]
```

| Flag                | Short | Description                      |
| ------------------- | ----- | -------------------------------- |
| `--json`            | `-j`  | Output JSON                      |
| `--xml`             |       | Output XML                       |
| `--exclude`         | `-e`  | Exclude folders                  |
| `--ignore`          | `-i`  | Ignore directories matching glob |
| `--min-rank`        | `-n`  | Minimum complexity rank (A-F)    |
| `--max-rank`        | `-x`  | Maximum complexity rank (A-F)    |
| `--average`         | `-a`  | Show average complexity          |
| `--total-average`   |       | Show total average complexity    |
| `--show-complexity` | `-s`  | Show complexity score with rank  |
| `--order`           | `-o`  | Ordering (score, lines, alpha)   |
| `--no-assert`       |       | Don't count assert statements    |
| `--fail-threshold`  |       | Exit code 1 if any block > N     |
| `--output-file`     | `-O`  | Save output to file              |

**Complexity Ranks:**
| Rank | Complexity | Description |
|------|------------|-------------|
| A | 1-5 | Low - simple |
| B | 6-10 | Medium - moderate |
| C | 11-20 | High - complex |
| D | 21-30 | Very high - refactor |
| E | 31-40 | Extremely high |
| F | 41+ | Critical - too complex |

---

### `hal` - Halstead Metrics

Calculate Halstead complexity metrics.

```bash
cytoscnpy hal [PATH] [OPTIONS]
```

| Flag            | Short | Description                       |
| --------------- | ----- | --------------------------------- |
| `--json`        | `-j`  | Output JSON                       |
| `--exclude`     | `-e`  | Exclude folders                   |
| `--ignore`      | `-i`  | Ignore directories matching glob  |
| `--functions`   | `-f`  | Compute metrics on function level |
| `--output-file` | `-O`  | Save output to file               |

**Halstead Metrics:**

- **h1**: Unique operators
- **h2**: Unique operands
- **N1**: Total operators
- **N2**: Total operands
- **vocabulary**: h1 + h2
- **length**: N1 + N2
- **volume**: length × log2(vocabulary)
- **difficulty**: (h1/2) × (N2/h2)
- **effort**: difficulty × volume
- **time**: effort / 18 seconds
- **bugs**: volume / 3000

---

### `mi` - Maintainability Index

Calculate Maintainability Index (0-100 scale).

```bash
cytoscnpy mi [PATH] [OPTIONS]
```

| Flag               | Short | Description                         |
| ------------------ | ----- | ----------------------------------- |
| `--json`           | `-j`  | Output JSON                         |
| `--exclude`        | `-e`  | Exclude folders                     |
| `--ignore`         | `-i`  | Ignore directories matching glob    |
| `--min-rank`       | `-n`  | Minimum MI rank (A-C)               |
| `--max-rank`       | `-x`  | Maximum MI rank (A-C)               |
| `--multi`          | `-m`  | Count multiline strings as comments |
| `--show`           | `-s`  | Show actual MI value                |
| `--average`        | `-a`  | Show average MI                     |
| `--fail-threshold` |       | Exit code 1 if any file MI < N      |
| `--output-file`    | `-O`  | Save output to file                 |

**MI Ranks:**
| Rank | MI Score | Description |
|------|----------|-------------|
| A | 20-100 | Highly maintainable |
| B | 10-19 | Moderately maintainable |
| C | 0-9 | Difficult to maintain |

---

## Configuration File

Create `.cytoscnpy.toml` in your project root:

```toml
[cytoscnpy]
# Core settings
confidence = 60
secrets = true
danger = true
quality = true
include_tests = false

# Quality thresholds
complexity = 10
nesting = 3
max_args = 5
max_lines = 50
min_mi = 40.0

# Path filters
exclude_folders = ["build", "dist", ".venv"]
include_folders = ["src"]

# CI/CD
fail_threshold = 5.0

# Advanced secret scanning
[cytoscnpy.secrets_config]
entropy_enabled = true
entropy_threshold = 4.0
min_length = 16
scan_comments = true
```

---

## Examples

### Basic Analysis

```bash
# Analyze current directory
cytoscnpy .

# Analyze specific files
cytoscnpy src/main.py src/utils.py

# Analyze multiple directories
cytoscnpy src/ lib/ scripts/

# Full security scan
cytoscnpy . --secrets --danger --quality

# Higher confidence threshold (fewer false positives)
cytoscnpy . --confidence 80

# Include test files in analysis
cytoscnpy . --include-tests

# Analyze with Jupyter notebooks
cytoscnpy . --include-ipynb --ipynb-cells
```

### Output Formats

```bash
# JSON output for CI/CD or further processing
cytoscnpy . --json > report.json

# Verbose mode for debugging
cytoscnpy . --verbose

# Quiet mode for clean CI output
cytoscnpy . --quiet

# Combine: quiet mode with JSON for logs
cytoscnpy . --quiet 2>&1 | tee output.log
```

### Path Filtering

```bash
# Exclude specific folders
cytoscnpy . --exclude-folder venv --exclude-folder build --exclude-folder node_modules

# Include specific folder (overrides defaults)
cytoscnpy . --include-folder vendor

# Analyze only src directory
cytoscnpy src/

# Exclude multiple patterns
cytoscnpy . --exclude-folder test --exclude-folder docs --exclude-folder migrations
```

### CI/CD Integration

```bash
# Basic quality gate: fail if >5% unused code
cytoscnpy . --fail-threshold 5

# Complexity gate: fail if any function >10
cytoscnpy . --max-complexity 10

# Maintainability gate: fail if MI <40
cytoscnpy . --min-mi 40

# All gates combined with quiet mode
cytoscnpy . --fail-threshold 5 --max-complexity 10 --min-mi 40 --quiet

# GitHub Actions with JSON report
cytoscnpy . --fail-threshold 5 --json > report.json
echo "Exit code: $?"

# Pre-commit hook style
cytoscnpy src/modified_file.py --fail-threshold 0 --quiet
```

### GitHub Actions Example

```yaml
# .github/workflows/code-quality.yml
name: Code Quality
on: [push, pull_request]

jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install CytoScnPy
        run: pip install cytoscnpy

      - name: Run Quality Gate
        run: |
          cytoscnpy . \
            --fail-threshold 5 \
            --max-complexity 15 \
            --min-mi 30 \
            --quality \
            --quiet
```

### Pre-commit Hook

```yaml
# .pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: cytoscnpy
        name: CytoScnPy Dead Code Check
        entry: cytoscnpy
        args: [--fail-threshold, "5", --quiet]
        language: python
        types: [python]
        pass_filenames: true
```

### Metric Subcommands

```bash
# Raw metrics for a project
cytoscnpy raw . --summary

# Save raw metrics to file
cytoscnpy raw . --json --output-file metrics.json

# Cyclomatic complexity - show only complex functions (rank C+)
cytoscnpy cc . --min-rank C --show-complexity

# Cyclomatic complexity - fail if any function >15
cytoscnpy cc . --fail-threshold 15

# Cyclomatic complexity - average for project
cytoscnpy cc . --average --total-average

# Halstead metrics per function
cytoscnpy hal . --functions --json

# Maintainability Index with actual values
cytoscnpy mi . --show --average

# MI gate: fail if any file <40
cytoscnpy mi . --fail-threshold 40

# MI showing only poor maintainability (rank C)
cytoscnpy mi . --max-rank C --show
```

### Security Scans

```bash
# Scan for secrets (API keys, passwords)
cytoscnpy . --secrets

# Scan for dangerous code (eval, exec, SQL injection)
cytoscnpy . --danger

# Full security audit
cytoscnpy . --secrets --danger --quality --json > security_report.json

# Strict mode: fail on any security issue
cytoscnpy . --secrets --danger --fail-on-quality
```

### Combining Options

```bash
# Full analysis with all features
cytoscnpy . \
  --secrets \
  --danger \
  --quality \
  --include-tests \
  --confidence 70 \
  --fail-threshold 10 \
  --max-complexity 15 \
  --verbose

# Production CI pipeline
cytoscnpy . \
  --fail-threshold 5 \
  --max-complexity 10 \
  --min-mi 40 \
  --secrets \
  --danger \
  --quiet \
  --json > report.json
```
