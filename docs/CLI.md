# CLI Reference

This document provides a comprehensive reference for the `cytoscnpy` command-line interface.

## Command Syntax

```bash
cytoscnpy [OPTIONS] [COMMAND]
```

## Main Options

### Input & Output

- `[paths]`: One or more paths to analyze (files or directories). If omitted, CytoScnPy defaults to the current working directory.
- `--root <PATH>`: Explicitly sets the project root. This is **highly recommended for CI/CD environments** to ensure that path-based security containment is correctly applied and that relative imports are resolved consistently. When `--root` is used, positional `[paths]` are not allowed.
- `--exclude-folder <FOLDER>`: Specifies a folder name to skip during analysis (e.g., `venv`, `node_modules`). This flag can be provided multiple times to exclude multiple directories.
- `--include-folder <FOLDER>`: Force-includes a folder that might otherwise be ignored by default rules (like some hidden directories).
- `--json`: Format the result as a raw JSON object. This is ideal for piping into tools like `jq` or for consumption by CI/CD scripts.
- `--verbose`, `-v`: Prints detailed logs during the analysis process, including which files are being scanned and any non-fatal issues encountered.
- `--quiet`: Minimalist output. Only the final summary table (or JSON) is displayed, suppressing the per-file findings table.
- `--fail-on-quality`: Causes the process to exit with code `1` if _any_ code quality issues (like high complexity or deep nesting) are detected.
- `--html`: Generates a self-contained, interactive HTML report. Note that this feature may require additional dependencies and automatically enables quality scanning.

### Scan Types

- `--secrets`, `-s`: Actively scans for high-entropy strings, API keys, and hardcoded credentials. It checks variables, strings, and even comments (depending on configuration).
- `--danger`, `-d`: Enables security scanning for dangerous patterns like `eval()`, `exec()`, and insecure temporary file creation. It also activates **taint analysis** to track user-controlled data flowing into dangerous sinks (e.g., SQL injection or command injection points).
- `--quality`, `-q`: Runs code quality checks including Cyclomatic Complexity, Maintainability Index, block nesting depth, and function length/argument counts.
- `--no-dead`, `-n`: Skips the default dead code detection. Use this if you only care about security vulnerabilities or quality metrics and want to speed up the analysis.

### Analysis Configuration

- `--confidence <N>`: Sets a minimum confidence threshold (0-100). CytoScnPy uses a scoring system for dead code; setting this to `80`, for example, will suppress "noisy" findings where the tool isn't certain the code is unused.
- `--include-tests`: By default, CytoScnPy ignores files in folders like `tests/` or `test/` starting from version 1.2.2. Use this flag to include them in the analysis.
- `--include-ipynb`: Enables scanning of Jupyter Notebook files. CytoScnPy extracts the Python code from cells and analyzes it as a virtual module.
- `--ipynb-cells`: When combined with `--include-ipynb`, this reports findings with cell numbers instead of just line numbers, making it easier to locate issues in the Notebook UI.
- `--clones`: Activates **duplicate code detection**. It uses AST-based hashing to find code blocks that are identical or nearly identical across your codebase.
- `--clone-similarity <N>`: Sets the similarity threshold for clone detection (0.0 to 1.0). A value of `1.0` finds only exact duplicates; a lower value like `0.8` (default) finds similar logic that might be refactored.
- `--fix`: Enables "Dead Code Auto-Fix" mode. By default, this is a **dry-run**—it will show you exactly what code would be removed without touching your files.
- `--apply`, `-a`: Executes the changes suggested by `--fix`. **Warning: This modifies your source code.** It is highly recommended to run with `--fix` first to review changes, and to have a clean Git state before applying.

### Quality Thresholds (Gate Overrides)

These flags allow you to set strict "gates" for your code. If any part of the codebase exceeds these numbers, CytoScnPy will exit with code `1`.

- `--fail-threshold <N>`: Exit with 1 if the total percentage of unused code exceeds `N`.
- `--max-complexity <N>`: Sets the maximum allowed Cyclomatic Complexity (standard is often `10`).
- `--min-mi <N>`: Sets the minimum allowed Maintainability Index (usually `40-65`).
- `--max-nesting <N>`: Sets the maximum allowed indentation/nesting level (e.g., `3` or `4`).
- `--max-args <N>`: Sets the maximum number of arguments a function can have.
- `--max-lines <N>`: Sets the maximum number of lines a function can have.

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
max_complexity = 10        # Max cyclomatic complexity
max_nesting = 3            # Max nesting depth
max_args = 5               # Max function arguments
max_lines = 50             # Max function lines
min_mi = 40.0              # Min Maintainability Index

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
