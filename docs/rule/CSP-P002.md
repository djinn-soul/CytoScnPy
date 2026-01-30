# CSP-P002: File Read Loads Entire File

**Category:** `Performance`

**Severity:** `MEDIUM`

## Description

Calling read() or readlines() loads the entire file into memory. Prefer streaming iteration for large files.

## Vulnerable Code Example

```python
with open(path) as f:
    data = f.read()
```

## Safer Code Example

```python
with open(path) as f:
    for line in f:
        handle(line)
```

## How to Suppress a Finding

```python
# ignore
# or
# noqa: CSP-P002
```
