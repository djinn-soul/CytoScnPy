# CSP-C304: Function Too Long

**Category:** `Quality`

**Severity:** `LOW`

## Description

This rule flags functions with too many lines. Long functions are harder to test, review, and change safely.

## Trigger Example

```python
def process_order(order):
    # ... many screens of logic here ...
    # parsing, validation, pricing, persistence, notifications, retries
    return True
```

## Recommended Refactor

```python
def process_order(order):
    normalized = normalize_order(order)
    validated = validate_order(normalized)
    total = calculate_total(validated)
    persist_order(validated, total)
    send_notifications(validated)
    return True
```

## How to Suppress a Finding

```python
# ignore
# noqa: CSP-C304
```
