# CLI Reference

This document provides a comprehensive reference for the `cytoscnpy` command-line interface.

## Command Syntax

```bash
cytoscnpy [OPTIONS] [COMMAND]
```

## Main Options

### Input & Output

- `[paths]`: Paths to analyze (files or directories). Defaults to current directory (`.`) if not specified.
- `--exclude-folder <FOLDER>`: Folders to exclude from analysis.
- `--include-folder <FOLDER>`: Folders to force-include in analysis (overrides default exclusions).
- `--json`: Output raw JSON.
- `--verbose`, `-v`: Enable verbose output for debugging.
- `--quiet`: Quiet mode (summary only).
- `--fail-on-quality`: Exit with code 1 if any quality issues are found.
- `--html`: Generate HTML report (requires `html_report` feature).

### Scan Types

- `--secrets`, `-s`: Scan for API keys/secrets.
- `--danger`, `-d`: Scan for dangerous code (includes taint analysis).
- `--quality`, `-q`: Scan for code quality issues.
- `--no-dead`, `-n`: Skip dead code detection (run only security/quality scans).

### Analysis Configuration

- `--confidence <N>`: Confidence threshold (0-100). Only findings with higher confidence are reported.
- `--include-tests`: Include test files in analysis.
- `--include-ipynb`: Include IPython Notebooks (.ipynb) in analysis.
- `--ipynb-cells`: Report findings at cell level for notebooks.
- `--clones`: Enable code clone detection.
- `--clone-similarity <N>`: Minimum similarity threshold (0.0-1.0, default 0.8).
- `--fix`: Auto-fix detected dead code (dry-run by default).
- `--apply`, `-a`: Apply fixes to files (use with `--fix`).

### Quality Thresholds (Overrides)

- `--fail-threshold <N>`: Exit 1 if finding percentage > N.
- `--max-complexity <N>`: Set maximum allowed Cyclomatic Complexity.
- `--min-mi <N>`: Set minimum allowed Maintainability Index.
- `--max-nesting <N>`: Set maximum allowed nesting depth.
- `--max-args <N>`: Set maximum allowed function arguments.
- `--max-lines <N>`: Set maximum allowed function lines.

## Subcommands

### `raw`

Calculate raw metrics (LOC, LLOC, SLOC, Comments, Multi, Blank).

```bash
cytoscnpy raw [OPTIONS] <PATH>
```

- `-j`, `--json`: Output JSON.
- `-s`, `--summary`: Show summary.
- `-O`, `--output-file <FILE>`: Save output to file.

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

### `hal`

Calculate Halstead Metrics.

```bash
cytoscnpy hal [OPTIONS] <PATH>
```

- `-f`, `--functions`: Compute metrics on function level.
- `-j`, `--json`: Output JSON.

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

### `stats`

Generate comprehensive project statistics report.

```bash
cytoscnpy stats [OPTIONS] <PATH>
```

- `-a`, `--all`: Enable all analysis: secrets, danger, quality, files.
- `-s`, `--secrets`: Scan for secrets.
- `-d`, `--danger`: Scan for dangerous code.
- `-q`, `--quality`: Scan for quality issues.
- `-j`, `--json`: Output JSON.
- `-o`, `--output <FILE>`: Output file path.
- `--exclude-folder <DIR>`: Exclude specific folders from analysis.

### `files`

Show per-file metrics table.

```bash
cytoscnpy files [OPTIONS] <PATH>
```

- `-j`, `--json`: Output only JSON.
- `--exclude-folder <DIR>`: Exclude specific folders from analysis.

### `mcp-server`

Start MCP server for LLM integration.

```bash
cytoscnpy mcp-server
```

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
# Note: include_ipynb and ipynb_cells are CLI-only options

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
```

## Exit Codes

- `0`: Success, no issues found (or issues below threshold).
- `1`: Issues found exceeding thresholds (quality, security, or fail_threshold).
- `2`: Runtime error or invalid arguments.

## See Also

- [Usage Guide](usage.md)
