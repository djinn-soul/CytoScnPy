# CSP-R003: Transitive Dependency

**Category:** `Dependency Hygiene`

**Severity:** `MEDIUM`

## Description

Source code imports a package that is present only because another dependency brings it in transitively. Declare it directly so installs remain stable if the parent dependency changes.

## Example

```python
import certifi
```

`certifi` appears in `uv.lock` or `poetry.lock`, but not in direct project dependencies.

## Fix

Add the imported package to direct dependencies.
