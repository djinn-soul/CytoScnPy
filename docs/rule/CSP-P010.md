# CSP-P010: Global Usage in Loop

**Category:** `Performance`

**Severity:** `LOW`

## Description

Accessing module-level constants inside loops can be slower than local variables. Hoist to a local.

## Vulnerable Code Example

```python
RATE = 1.25
for x in items:
    total += x * RATE
```

## Safer Code Example

```python
RATE = 1.25
rate = RATE
for x in items:
    total += x * rate
```

## How to Suppress a Finding

```python
# ignore
# or
# noqa: CSP-P010
```
