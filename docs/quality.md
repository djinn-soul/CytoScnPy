# Code Quality Rules

CytoScnPy detects common Python code quality issues and maintains alignment with industry-standard linting codes (Flake8, Bugbear, Pylint).

Enable quality analysis with the `--quality` flag:

```bash
cytoscnpy . --quality
```

---

## Best Practices

| Name                         | Description                                                                   |
| :--------------------------- | :---------------------------------------------------------------------------- |
| `MutableDefaultArgumentRule` | Detects mutable default arguments (lists, dicts, sets).                       |
| `BareExceptRule`             | Detects `except:` blocks without a specific exception class.                  |
| `DangerousComparisonRule`    | Detects comparisons to `True`, `False`, or `None` using `==` instead of `is`. |

## Maintainability & Complexity

| Name                 | Description                                                |
| :------------------- | :--------------------------------------------------------- |
| `ComplexityRule`     | Function cyclomatic complexity (McCabe) exceeds threshold. |
| `NestingRule`        | Code block is nested too deeply.                           |
| `ArgumentCountRule`  | Function has too many arguments.                           |
| `FunctionLengthRule` | Function is too long (lines of code).                      |

---

## Configuration

You can tune the thresholds for quality rules in your `.cytoscnpy.toml`:

```toml
[cytoscnpy]
max_complexity = 15    # Default: 10
max_nesting = 4        # Default: 3
max_args = 6           # Default: 5
max_lines = 100        # Default: 50
```

## Suppression

Use the standard `# noqa` syntax to suppress specific findings. Note that Rule IDs for quality checks are currently under migration; use generic `# noqa` to suppress all quality findings on a line:

```python
def my_function(arg=[]):  # noqa
    pass

try:
    do_something()
except:  # noqa
    pass
```
