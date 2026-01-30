# CSP-L001: Mutable Default Arguments

**Category:** `Best Practices`

**Severity:** `MEDIUM`

## Description

Using a mutable object (list, dict, set) as a default argument can cause state to leak across calls because the default is evaluated once at function definition time.

## Vulnerable Code Example

```python
def add_item(item, items=[]):
    items.append(item)
    return items
```

## Safer Code Example

```python
def add_item(item, items=None):
    if items is None:
        items = []
    items.append(item)
    return items
```

## How to Suppress a Finding

```python
# ignore
# or
# noqa: CSP-L001
```
