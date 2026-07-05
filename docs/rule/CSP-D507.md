# CSP-D507: TOCTOU Race Condition

**Vulnerability Category:** `Filesystem`

**Severity:** `MEDIUM`

## Description

This rule flags filesystem time-of-check/time-of-use patterns, such as checking `os.path.exists()` or `Path.exists()` before opening the same path.

Between the check and the use, another process can replace the file, swap a symlink, or alter permissions. This can lead to reading or writing the wrong file, overwriting sensitive data, or bypassing intended checks.

## Vulnerable Code Example

```python
import os

def read_config(path):
    if os.path.exists(path):
        with open(path) as handle:
            return handle.read()
    return ""
```

The file can change after `exists()` returns but before `open()` runs.

## Safe Code Example

```python
def read_config(path):
    try:
        with open(path) as handle:
            return handle.read()
    except FileNotFoundError:
        return ""
```

Use the operation directly and handle the exception. For writes, use atomic creation modes and secure file APIs where possible.

## How to Suppress a Finding

Suppress only when the path is not attacker-influenced and the race cannot cross a security boundary.

```python
# path is an internal temporary file in a private directory.
# ignore: CSP-D507
if os.path.exists(path):
    open(path).close()
```
