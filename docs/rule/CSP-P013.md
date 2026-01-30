# CSP-P013: Comprehension Suggestion

**Category:** `Performance`

**Severity:** `MEDIUM`

## Description

Simple loops building lists, sets, or dicts can be replaced with comprehensions.

## Vulnerable Code Example

```python
result = []
for item in items:
    result.append(item.id)
```

## Safer Code Example

```python
result = [item.id for item in items]
```

## How to Suppress a Finding

```python
# ignore
# or
# noqa: CSP-P013
```
