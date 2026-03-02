# CSP-S300: Suspicious Hardcoded Secret Assignment

**Category:** `Secrets`

**Severity:** `MEDIUM`

## Description

This rule reports suspicious assignments where variable names suggest credentials (for example `password`, `token`, `secret`) and the value appears hardcoded.

## Trigger Example

```python
db_password = "admin123!"
jwt_secret = "my-local-dev-secret"
```

## Recommended Refactor

```python
import os

db_password = os.environ["DB_PASSWORD"]
jwt_secret = os.environ["JWT_SECRET"]
```

## Notes

- This rule complements `CSP-S200` by catching non-patterned secrets.
- Entropy and suspicious-name scoring affect confidence.

## How to Suppress a Finding

```python
# ignore
# noqa: CSP-S300
```
