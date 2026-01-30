# CSP-P012: Use Tuple for Immutable Constant List

**Category:** `Performance`

**Severity:** `LOW`

## Description

Immutable constant lists can use tuples to reduce overhead and signal immutability.

## Vulnerable Code Example

```python
ALLOWED = ["a", "b", "c"]
```

## Safer Code Example

```python
ALLOWED = ("a", "b", "c")
```

## How to Suppress a Finding

```python
# ignore
# or
# noqa: CSP-P012
```
