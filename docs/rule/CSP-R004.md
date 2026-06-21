# CSP-R004: Development Dependency Used in Production

**Category:** `Dependency Hygiene`

**Severity:** `MEDIUM`

## Description

Production source code imports a package declared only as a development dependency. Move the package to production dependencies or move the import into test/dev-only code.

## Example

```python
import pytest
```

```toml
[dependency-groups]
dev = ["pytest"]
```

## Fix

If production code really needs the package, declare it as a production dependency.
