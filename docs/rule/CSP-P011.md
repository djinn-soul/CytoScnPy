# CSP-P011: Use memoryview for Bytes Slicing in Loop

**Category:** `Performance`

**Severity:** `LOW`

## Description

Slicing bytes repeatedly creates copies. Use memoryview for zero-copy slicing.

## Vulnerable Code Example

```python
for i in range(0, len(data), 4):
    chunk = data[i:i+4]
    handle(chunk)
```

## Safer Code Example

```python
view = memoryview(data)
for i in range(0, len(view), 4):
    chunk = view[i:i+4]
    handle(chunk)
```

## How to Suppress a Finding

```python
# ignore
# or
# noqa: CSP-P011
```
