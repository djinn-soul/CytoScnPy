# Code Quality Rules

CytoScnPy detects common Python code quality issues and maintains alignment with industry-standard linting codes (Flake8, Bugbear, Pylint).

Enable quality analysis with the `--quality` flag:

```bash
cytoscnpy . --quality
```

---

## Rule ID Prefixes

| Prefix   | Category                              |
| :------- | :------------------------------------ |
| `CSP-L`  | Best Practices                        |
| `CSP-Q`  | Complexity & Maintainability Metrics  |
| `CSP-C`  | Maintainability Limits (Args/Length)  |
| `CSP-P`  | Performance                           |

> Individual rule pages are available for **Best Practices** and **Performance**. Maintainability rules are summarized in this index.

## Best Practices

| ID                              | Rule                          | Description                                                                   |
| :------------------------------ | :---------------------------- | :---------------------------------------------------------------------------- |
| [CSP-L001](rule/CSP-L001.md)    | `MutableDefaultArgumentRule`  | Detects mutable default arguments (lists, dicts, sets).                       |
| [CSP-L002](rule/CSP-L002.md)    | `BareExceptRule`              | Detects `except:` blocks without a specific exception class.                  |
| [CSP-L003](rule/CSP-L003.md)    | `DangerousComparisonRule`     | Detects comparisons to `True`, `False`, or `None` using `==` instead of `is`. |

## Maintainability & Complexity

| ID          | Rule                         | Description                                                                 | Config                                |
| :---------- | :--------------------------- | :-------------------------------------------------------------------------- | :------------------------------------ |
| `CSP-Q301`  | `ComplexityRule`             | Function cyclomatic complexity (McCabe) exceeds threshold.                  | `max_complexity`                      |
| `CSP-Q302`  | `NestingRule`                | Code block is nested too deeply.                                            | `max_nesting`                         |
| `CSP-Q303`  | Maintainability Index Gate   | File MI below threshold (only emitted when `min_mi` is set).                | `min_mi`                              |
| `CSP-Q304`  | `CognitiveComplexityRule`    | Cognitive complexity exceeds fixed threshold (default 15).                  | Fixed threshold (not configurable)    |
| `CSP-Q305`  | `CohesionRule`               | Class lacks cohesion (LCOM4 > 1).                                           | Fixed threshold (not configurable)    |
| `CSP-C303`  | `ArgumentCountRule`          | Function has too many arguments.                                            | `max_args`                            |
| `CSP-C304`  | `FunctionLengthRule`         | Function is too long (line count).                                          | `max_lines`                           |

## Performance

| ID                              | Rule                          | Description                                                                 |
| :------------------------------ | :---------------------------- | :-------------------------------------------------------------------------- |
| [CSP-P001](rule/CSP-P001.md)    | `MembershipInListRule`        | Membership test in list literal or list comprehension (use set).            |
| [CSP-P002](rule/CSP-P002.md)    | `FileReadMemoryRiskRule`      | `read()` / `readlines()` loads entire file into RAM.                        |
| [CSP-P003](rule/CSP-P003.md)    | `StringConcatInLoopRule`      | Accumulated `+` string concatenation inside loops.                          |
| [CSP-P004](rule/CSP-P004.md)    | `UselessCastRule`             | Unnecessary `list()`/`tuple()` wrapping `range()`/`map()`/`filter()`.        |
| [CSP-P005](rule/CSP-P005.md)    | `RegexLoopRule`               | `re.compile()` or `ast.parse()` called inside loops.                        |
| [CSP-P006](rule/CSP-P006.md)    | `AttributeChainHoistingRule`  | Deep attribute access inside loops (hoist to local).                        |
| [CSP-P007](rule/CSP-P007.md)    | `PureCallHoistingRule`        | Pure builtin calls in loops with invariant arguments.                       |
| [CSP-P008](rule/CSP-P008.md)    | `ExceptionFlowInLoopRule`     | `try/except` used as control flow in loops (KeyError/AttributeError).       |
| [CSP-P009](rule/CSP-P009.md)    | `IncorrectDictIteratorRule`   | `.items()` used while discarding key/value (use `.keys()`/`.values()`).      |
| [CSP-P010](rule/CSP-P010.md)    | `GlobalUsageInLoopRule`       | Global/constant usage inside loops (hoist to local).                        |
| [CSP-P011](rule/CSP-P011.md)    | `MemoryviewOverBytesRule`     | Slicing bytes in loops (use `memoryview()` for zero-copy).                  |
| [CSP-P012](rule/CSP-P012.md)    | `UseTupleOverListRule`        | Constant list literal is immutable (prefer tuple).                          |
| [CSP-P013](rule/CSP-P013.md)    | `ComprehensionSuggestionRule` | Loop can be replaced by list/set/dict comprehension.                        |
| [CSP-P015](rule/CSP-P015.md)    | `PandasChunksizeRiskRule`     | `pandas.read_csv()` without `chunksize`/`nrows`/`iterator`.                 |

---

## Configuration

You can tune the thresholds for quality rules in your `.cytoscnpy.toml`:

```toml
[cytoscnpy]
max_complexity = 15    # Default: 10
max_nesting = 4        # Default: 3
max_args = 6           # Default: 5
max_lines = 100        # Default: 50
min_mi = 40.0          # Default: unset (no MI gate)
```

## Suppression

Use standard suppression comments. You can suppress all findings or target specific rule IDs:

```python
def my_function(arg=[]):  # noqa
    pass

def risky_compare(x):  # noqa: CSP-L003
    return x == None

for x in items:  # noqa: CSP-P003
    out += x
```
