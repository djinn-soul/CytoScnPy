# CSP-P003: String Concatenation in Loop

**Category:** `Performance`

**Severity:** `LOW`

## Description

Repeated string concatenation in a loop can be quadratic. Use join().

## Vulnerable Code Example

```python
out = ""
for x in items:
    out += str(x)
```

## Safer Code Example

```python
out = "".join(str(x) for x in items)
```

## How to Suppress a Finding

```python
# ignore
# or
# noqa: CSP-P003
```
