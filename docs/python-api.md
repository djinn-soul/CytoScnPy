# Python API and pytest Plugin

The `cytoscnpy` package can be used from Python code and from pytest, as well
as from the command line. Both run the Rust analyzer in-process.

## Python API

```python
import cytoscnpy

result = cytoscnpy.scan("src", secrets=True, danger=True)
print(result["analysis_summary"]["total_files"])
for finding in result["danger"]:
    print(finding["rule_id"], finding["file"], finding["line"], finding["message"])
```

### `scan()`

Scan files and directories and return the findings as a dictionary.

```python
cytoscnpy.scan(
    paths=".",                # a path, or a list of files and directories
    *,
    confidence=None,          # 0-100; minimum confidence for dead-code findings
    secrets=None,             # scan for hardcoded secrets
    danger=None,              # scan for dangerous patterns and taint flows
    quality=None,             # scan for complexity and quality issues
    include_tests=None,       # include test files
    exclude_folders=None,     # folders to skip
    include_folders=None,     # folders to scan even if excluded by default
    include_ipynb=None,       # include Jupyter notebooks
    clones=None,              # run clone detection
    clone_similarity=None,    # 0.0-1.0, minimum clone similarity
) -> cytoscnpy.ScanResult
```

An option left as `None` is read from the project's `.cytoscnpy.toml` or
`pyproject.toml`, so a call behaves like the CLI run from that project.

Raises:

- `ValueError`: `confidence` is outside 0–100, or the project configuration is
  invalid.
- `RuntimeError`: a requested clone analysis failed, including an invalid
  `clone_similarity`.

### `scan_code()`

Scan a string of Python source without writing it to disk.

```python
result = cytoscnpy.scan_code(
    "x = eval(input())\n",
    filename="<stdin>",  # name used in findings
    confidence=60,
    secrets=True,
    danger=True,
    quality=True,
)
print([f["rule_id"] for f in result["danger"]])  # ['CSP-D001', ...]
```

### Result Fields

`scan()` and `scan_code()` return a `ScanResult` dictionary with the same fields
as the CLI's `--json` output:

| Field | Contents |
|---|---|
| `unused_functions`, `unused_methods`, `unused_classes`, `unused_imports`, `unused_variables`, `unused_parameters` | Dead code findings |
| `unused_dependencies`, `missing_dependencies`, `missing_dependency_details`, `transitive_dependencies`, `dev_dependencies_in_production`, `stdlib_dependencies` | Dependency findings |
| `secrets` | Secret findings (`CSP-S*`) |
| `danger` | Dangerous code findings (`CSP-D*`) |
| `taint_findings` | Source-to-sink taint flows |
| `quality` | Quality findings (`CSP-L*`, `CSP-Q*`, `CSP-P*`, ...) |
| `clones` | Clone findings, when clone detection is on |
| `parse_errors` | Files that could not be parsed |
| `file_metrics` | Per-file metrics |
| `analysis_summary` | Totals: files, lines, definitions, finding counts, average complexity and MI, raw and Halstead metrics |

### Raw JSON and CLI Functions

| Function | Returns |
|---|---|
| `cytoscnpy.scan_json(...)` | Same arguments as `scan()`, with `paths` as a list of strings; returns the JSON text |
| `cytoscnpy.scan_code_json(code, ...)` | Same arguments as `scan_code()`; returns the JSON text |
| `cytoscnpy.run(args)` | Runs the CLI with a list of arguments, for example `["src", "--json"]`, and returns the exit code |

## pytest Plugin

Installing `cytoscnpy` registers a pytest plugin. When enabled, it scans the
project once at the start of the session and adds one `[cytoscnpy]` test item
per Python file. An item fails when CytoScnPy reports findings for that file,
and its failure output lists them.

Enable it for one run:

```bash
pytest --cytoscnpy
```

Or enable it permanently in `pyproject.toml` (or `pytest.ini` / `setup.cfg`):

```toml
[tool.pytest.ini_options]
cytoscnpy = true
cytoscnpy_path = "src/"   # optional; relative to the pytest root, default "."
```

| Setting | Type | Default | Meaning |
|---|---|---|---|
| `--cytoscnpy` | command-line flag | off | Enable the plugin for this run |
| `cytoscnpy` | ini, bool | `false` | Enable the plugin on every run |
| `cytoscnpy_path` | ini, string | `"."` | Path to scan, relative to the pytest root |

The scan uses the project's normal CytoScnPy configuration, so
`.cytoscnpy.toml` or `[tool.cytoscnpy]` decides which checks run.

If CytoScnPy itself fails (for example, an invalid custom secret regex in the
config), every item is reported as an **error** rather than a failure, and the
pytest session exits with a failing status. This keeps a broken analyzer run
from looking like a clean one.

Select or skip the items with pytest's usual filters:

```bash
pytest --cytoscnpy -k cytoscnpy        # only the CytoScnPy items
pytest --cytoscnpy -k "not cytoscnpy"  # only your tests
```
