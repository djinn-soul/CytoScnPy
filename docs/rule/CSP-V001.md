# CSP-V001: Unused Code

**Category:** `Dead Code`

## Description

This rule reports definitions that nothing in the project uses: functions, methods, classes, imports, variables, and parameters. Dead code is reported by default; pass `--no-dead` (`-n`) to skip it.

Each finding has a confidence score from 0 to 100. Names that may be used dynamically, such as methods of a class extending a base from outside the project, receive lower confidence or are not reported. Use `--confidence <N>` to hide findings below a threshold.

## Finding Kinds

Every kind can be suppressed with `CSP-V001` or with its own ID. Imports and variables also accept the matching Ruff/Flake8 code.

| Kind | Reported when | Suppression codes |
| :--- | :------------ | :---------------- |
| Function | A function is never called or referenced | `CSP-V001`, `unused-function` |
| Method | A method is never called or referenced; methods of an unused class are reported too | `CSP-V001`, `unused-method` |
| Class | A class is never instantiated, subclassed, or referenced | `CSP-V001`, `unused-class` |
| Import | An imported name is never used or re-exported | `CSP-V001`, `unused-import`, `F401` |
| Variable | A variable is assigned but never read | `CSP-V001`, `unused-variable`, `F841` |
| Parameter | A function parameter is never used | `CSP-V001`, `unused-parameter` |

## Trigger Example

```python
import os
import sys


def main():
    print(sys.argv)


def legacy_helper():
    return 42
```

`import os` and `legacy_helper` are reported: nothing uses them.

## Recommended Refactor

```python
import sys


def main():
    print(sys.argv)
```

Preview the removal with `cytoscnpy . --fix`, then apply it with `cytoscnpy . --fix --apply`. Each suggested fix is checked to leave valid Python.

## What Counts as Used

- Calls, references stored in registries or passed as callbacks, and returned nested functions.
- Names listed in `__all__`, and redundant aliases such as `import x as x` (an explicit re-export under PEP 484).
- Names used only in type annotations, including string forward references such as `x: "OrderedDict[str, int]"`.
- Decorators, default arguments, and metaclasses, which run when the definition runs.

## How to Suppress a Finding

```python
import os  # noqa: CSP-V001
import json  # noqa: F401


def plugin_hook():  # noqa: unused-function
    ...
```

To keep a set of intentionally unused names without inline comments, generate a whitelist:

```bash
cytoscnpy . --make-whitelist > whitelist.py
cytoscnpy . --whitelist whitelist.py
```

See [Common Issues: False Positives](../usage.md#common-issues) for the full `# noqa` syntax.
