# CSP-R005: Standard Library Dependency

**Category:** `Dependency Hygiene`

**Severity:** `LOW`

## Description

A Python standard-library module is declared as a third-party dependency. Standard-library modules do not need package declarations.

## Example

```toml
[project]
dependencies = ["asyncio"]
```

## Fix

Remove the standard-library module from project dependencies.
