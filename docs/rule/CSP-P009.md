# CSP-P009: Incorrect Dictionary Iterator

**Category:** `Performance`

**Severity:** `LOW`

## Description

Using .items() while discarding key or value wastes work. Use .keys() or .values().

## Vulnerable Code Example

```python
for _, value in data.items():
    handle(value)
```

## Safer Code Example

```python
for value in data.values():
    handle(value)
```

## How to Suppress a Finding

```python
# ignore
# or
# noqa: CSP-P009
```
