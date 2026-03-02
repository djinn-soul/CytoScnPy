# CSP-S200: Secret Pattern Match

**Category:** `Secrets`

**Severity:** `HIGH`

## Description

This rule reports hardcoded credentials that match known secret/token patterns (for example API keys, access tokens, and private-key fragments).

## Trigger Example

```python
GITHUB_TOKEN = "ghp_1234567890abcdefghijklmnopqrstuvwxyz"
```

## Recommended Refactor

```python
import os

GITHUB_TOKEN = os.environ["GITHUB_TOKEN"]
```

## Notes

- This rule is regex and pattern driven.
- Custom patterns can be added in `secrets_config.patterns`.

## How to Suppress a Finding

```python
# ignore
# noqa: CSP-S200
```
