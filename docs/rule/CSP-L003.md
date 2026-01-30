# CSP-L003: Dangerous Comparison to True/False/None

**Category:** `Best Practices`

**Severity:** `LOW`

## Description

Comparing to True/False/None with == or != can be error-prone. Use 'is' or 'is not' for identity checks.

## Vulnerable Code Example

```python
if value == None:
    return True
```

## Safer Code Example

```python
if value is None:
    return True
```

## How to Suppress a Finding

```python
# ignore
# or
# noqa: CSP-L003
```
