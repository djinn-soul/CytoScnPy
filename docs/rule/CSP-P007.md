# CSP-P007: Pure Call in Loop

**Category:** `Performance`

**Severity:** `LOW`

## Description

Pure builtin calls with invariant arguments inside loops can be hoisted.

## Vulnerable Code Example

```python
for x in items:
    limit = len(items)
    handle(x, limit)
```

## Safer Code Example

```python
limit = len(items)
for x in items:
    handle(x, limit)
```

## How to Suppress a Finding

```python
# ignore
# or
# noqa: CSP-P007
```
