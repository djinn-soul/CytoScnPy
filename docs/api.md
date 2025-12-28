# Python API

CytoScnPy exposes a minimal Python API to invoke the analyzer programmatically.

## `cytoscnpy` module

### `run(args: List[str]) -> int`

Executes the CytoScnPy analyzer with the provided command-line arguments.

**Parameters:**

- `args`: A list of strings representing command-line arguments (e.g., `[".", "--json"]`).

**Returns:**

- `int`: Exit code (0 for success, non-zero for failure/findings).

**Example:**

```python
import cytoscnpy
import sys

# Run analysis on current directory and output JSON
args = [".", "--json", "--quiet"]
exit_code = cytoscnpy.run(args)

if exit_code != 0:
    print("Issues found!")
```

> **Note**: The `run` function writes output directly to standard output/error, similar to running the CLI. To capture output, you may need to redirect stdout.
