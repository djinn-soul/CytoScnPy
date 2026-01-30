# CSP-P005: Regex Compile or ast.parse in Loop

**Category:** `Performance`

**Severity:** `MEDIUM`

## Description

Compiling regex or parsing AST inside loops is expensive. Move it outside the loop and reuse.

## Vulnerable Code Example

```python
for item in items:
    pat = re.compile("^[a-z]+$")
    if pat.match(item):
        handle(item)
```

## Safer Code Example

```python
pat = re.compile("^[a-z]+$")
for item in items:
    if pat.match(item):
        handle(item)
```

## How to Suppress a Finding

```python
# ignore
# or
# noqa: CSP-P005
```
