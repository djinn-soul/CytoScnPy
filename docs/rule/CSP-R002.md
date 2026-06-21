# CSP-R002: Unused Dependency

**Category:** `Dependency Hygiene`

**Severity:** `LOW`

## Description

A production dependency is declared but no matching import was found in scanned source files. Remove it if it is not required at runtime.

Development dependencies are skipped by default to avoid noise from generated dev exports. Use `cytoscnpy deps --include-dev-unused` for strict dev-dependency checks.

## Example

```toml
[project]
dependencies = ["unused-package"]
```

## Fix

Remove the dependency from the project metadata or requirements file if it is no longer needed.
