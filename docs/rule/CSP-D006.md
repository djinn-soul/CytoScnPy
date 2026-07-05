# CSP-D006: Privilege Escalation via Process Credential Changes

**Vulnerability Category:** `Code Execution`

**Severity:** `HIGH`

## Description

This rule flags calls that change process user or group credentials, such as `os.setuid()`, `os.setgid()`, `os.setreuid()`, `os.setregid()`, `os.seteuid()`, `os.setegid()`, and `os.setgroups()`.

Changing process credentials is security-sensitive. If attacker-controlled input or unsafe control flow can influence these calls, the application may run with the wrong privileges, retain elevated access longer than intended, or drop privileges incorrectly.

## Vulnerable Code Example

```python
import os

def switch_user(uid):
    os.setuid(uid)
```

The caller controls the target user ID. In a privileged process, this can create an unintended privilege boundary bypass.

## Safe Code Example

```python
import os

SERVICE_UID = 1001

def drop_privileges():
    os.setgid(SERVICE_UID)
    os.setuid(SERVICE_UID)
```

Only change credentials to fixed, expected service identities during controlled startup code. Avoid accepting user-controlled IDs.

## How to Suppress a Finding

Suppress this rule only when the credential change is intentional, isolated, and reviewed as part of process startup or sandbox setup.

```python
# Drops privileges once during service startup.
# ignore: CSP-D006
os.setuid(SERVICE_UID)
```
