# CSP-Q303: Maintainability Index Below Minimum

**Category:** `Quality`

**Severity:** `MEDIUM`

## Description

This rule reports files whose Maintainability Index (MI) is below the configured `min_mi` threshold.

## Trigger Example

```python
# Large, highly complex module with many branches and low readability.
# CytoScnPy computes MI and emits CSP-Q303 when below configured minimum.
```

## Recommended Refactor

- Split large modules into focused units.
- Reduce branching complexity.
- Improve naming and remove dead paths.
- Add tests before refactoring risky sections.

## Configuration

```toml
[cytoscnpy]
min_mi = 40.0
```

## How to Suppress a Finding

```python
# ignore
# noqa: CSP-Q303
```
