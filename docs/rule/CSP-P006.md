# CSP-P006: Deep Attribute Access in Loop

**Category:** `Performance`

**Severity:** `LOW`

## Description

Repeated deep attribute access inside loops can be hoisted into a local variable.

## Vulnerable Code Example

```python
for item in items:
    value = item.user.profile.settings.theme
    handle(value)
```

## Safer Code Example

```python
for item in items:
    settings = item.user.profile.settings
    handle(settings.theme)
```

## How to Suppress a Finding

```python
# ignore
# or
# noqa: CSP-P006
```
