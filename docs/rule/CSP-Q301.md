# CSP-Q301: Cyclomatic Complexity Threshold Exceeded

**Category:** `Quality`

**Severity:** `MEDIUM`

## Description

This rule reports functions with cyclomatic complexity above `max_complexity`. High complexity increases bug risk and makes behavior harder to reason about.

## Trigger Example

```python
def route_event(event):
    if event.kind == "A":
        if event.priority > 10:
            return "x"
        return "y"
    elif event.kind == "B":
        if event.retry and not event.cancelled:
            return "z"
    elif event.kind == "C":
        return "k"
    return "default"
```

## Recommended Refactor

```python
def route_event(event):
    handlers = {
        "A": _handle_a,
        "B": _handle_b,
        "C": _handle_c,
    }
    return handlers.get(event.kind, _handle_default)(event)
```

## Configuration

```toml
[cytoscnpy]
max_complexity = 10
```

## How to Suppress a Finding

```python
# ignore
# noqa: CSP-Q301
```
