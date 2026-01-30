# CSP-P001: Membership Test in List

**Category:** `Performance`

**Severity:** `MEDIUM`

## Description

Using 'in' with a list literal (or list comprehension) is O(N). Use a set for faster lookups, especially in loops.

## Vulnerable Code Example

```python
if x in [1, 2, 3, 4]:
    handle(x)
```

## Safer Code Example

```python
allowed = {1, 2, 3, 4}
if x in allowed:
    handle(x)
```

## How to Suppress a Finding

```python
# ignore
# or
# noqa: CSP-P001
```
