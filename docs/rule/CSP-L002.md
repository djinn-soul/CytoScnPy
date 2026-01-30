# CSP-L002: Bare Except

**Category:** `Best Practices`

**Severity:** `LOW`

## Description

A bare except catches SystemExit and KeyboardInterrupt and can hide real failures. Catch specific exceptions instead.

## Vulnerable Code Example

```python
try:
    do_work()
except:
    pass
```

## Safer Code Example

```python
try:
    do_work()
except (ValueError, IOError) as exc:
    handle_error(exc)
```

## How to Suppress a Finding

```python
# ignore
# or
# noqa: CSP-L002
```
