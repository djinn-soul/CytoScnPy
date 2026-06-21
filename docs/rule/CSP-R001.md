# CSP-R001: Missing Dependency

**Category:** `Dependency Hygiene`

**Severity:** `MEDIUM`

## Description

An imported third-party package is not declared as a direct project dependency. Declare direct imports in `pyproject.toml` or the relevant requirements file.

## Example

```python
import requests
```

```toml
[project]
dependencies = []
```

## Fix

```toml
[project]
dependencies = ["requests"]
```
