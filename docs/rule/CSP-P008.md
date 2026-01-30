# CSP-P008: Exception Flow Control in Loop

**Category:** `Performance`

**Severity:** `LOW`

## Description

Using try/except for common control flow in loops is slow. Prefer checks like in/get/hasattr.

## Vulnerable Code Example

```python
for key in keys:
    try:
        value = mapping[key]
    except KeyError:
        continue
```

## Safer Code Example

```python
for key in keys:
    if key not in mapping:
        continue
    value = mapping[key]
```

## How to Suppress a Finding

```python
# ignore
# or
# noqa: CSP-P008
```
