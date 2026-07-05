# CSP-D705: Hardcoded Default Credentials

**Vulnerability Category:** `Best Practices`

**Severity:** `HIGH`

## Description

This rule flags comparisons against well-known default credentials such as `admin`, `password`, or similar hardcoded account values.

Hardcoded default credentials are frequently discovered and reused by attackers. Even when they are intended for development, they often leak into production paths or become fallback authentication logic.

## Vulnerable Code Example

```python
def login(username, password):
    if username == "admin" and password == "password":
        return True
    return check_database(username, password)
```

Anyone who knows the default pair can bypass normal credential storage.

## Safe Code Example

```python
def login(username, password):
    return check_password_hash(load_password_hash(username), password)
```

Store credentials outside source code, hash passwords with a modern password hashing scheme, and remove development-only bypasses before production.

## How to Suppress a Finding

Suppress only for defensive checks that reject known bad defaults rather than accepting them.

```python
# Rejects a known unsafe default during setup.
# ignore: CSP-D705
if password == "password":
    raise ValueError("Choose a stronger password")
```
