# CSP-Q304: Cognitive Complexity Threshold Exceeded

**Category:** `Quality`

**Severity:** `MEDIUM`

## Description

This rule reports functions with high cognitive complexity. It measures how difficult code is to understand, emphasizing nested branches and flow interruptions.

## Trigger Example

```python
def resolve(items, mode):
    result = []
    for item in items:
        if item.enabled:
            if mode == "strict":
                if item.valid():
                    result.append(item)
                else:
                    if item.fallback:
                        result.append(item.fallback)
            else:
                result.append(item)
    return result
```

## Recommended Refactor

```python
def resolve(items, mode):
    if mode == "strict":
        return [_strict(item) for item in items if item.enabled]
    return [item for item in items if item.enabled]
```

## How to Suppress a Finding

```python
# ignore
# noqa: CSP-Q304
```
