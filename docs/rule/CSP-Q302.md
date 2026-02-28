# CSP-Q302: Excessive Nesting Depth

**Category:** `Quality`

**Severity:** `MEDIUM`

## Description

This rule reports blocks that exceed `max_nesting`. Deeply nested logic is difficult to read and often hides edge-case bugs.

## Trigger Example

```python
def authorize(user, request):
    if user:
        if user.active:
            if request.region in user.allowed_regions:
                if request.scope == "write":
                    return True
    return False
```

## Recommended Refactor

```python
def authorize(user, request):
    if not user or not user.active:
        return False
    if request.region not in user.allowed_regions:
        return False
    return request.scope == "write"
```

## Configuration

```toml
[cytoscnpy]
max_nesting = 3
```

## How to Suppress a Finding

```python
# ignore
# noqa: CSP-Q302
```
