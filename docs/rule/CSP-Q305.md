# CSP-Q305: Low Cohesion (LCOM4)

**Category:** `Quality`

**Severity:** `LOW`

## Description

This rule reports classes with low cohesion (high LCOM4). It indicates the class likely mixes unrelated responsibilities.

## Trigger Example

```python
class Service:
    def send_email(self, email):
        ...

    def calculate_tax(self, invoice):
        ...

    def create_backup(self, path):
        ...
```

## Recommended Refactor

```python
class NotificationService:
    def send_email(self, email):
        ...


class BillingService:
    def calculate_tax(self, invoice):
        ...


class BackupService:
    def create_backup(self, path):
        ...
```

## How to Suppress a Finding

```python
# ignore
# noqa: CSP-Q305
```
