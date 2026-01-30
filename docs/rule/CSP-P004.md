# CSP-P004: Unnecessary list()/tuple() Cast

**Category:** `Performance`

**Severity:** `LOW`

## Description

Wrapping range/map/filter with list() or tuple() is often unnecessary when iterating directly.

## Vulnerable Code Example

```python
for x in list(range(10)):
    handle(x)
```

## Safer Code Example

```python
for x in range(10):
    handle(x)
```

## How to Suppress a Finding

```python
# ignore
# or
# noqa: CSP-P004
```
