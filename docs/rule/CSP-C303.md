# CSP-C303: Too Many Function Arguments

**Category:** `Quality`

**Severity:** `LOW`

## Description

This rule flags functions or methods that accept too many parameters. Excessive argument count usually indicates mixed responsibilities and makes call sites harder to read and maintain.

## Trigger Example

```python
def build_user(first, last, email, role, team, region, timezone):
    return {
        "first": first,
        "last": last,
        "email": email,
        "role": role,
        "team": team,
        "region": region,
        "timezone": timezone,
    }
```

## Recommended Refactor

```python
from dataclasses import dataclass


@dataclass
class UserInput:
    first: str
    last: str
    email: str
    role: str
    team: str
    region: str
    timezone: str


def build_user(user: UserInput) -> dict[str, str]:
    return user.__dict__
```

## How to Suppress a Finding

```python
# ignore
# noqa: CSP-C303
```
