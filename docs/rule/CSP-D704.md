# CSP-D704: Call to Blacklisted Function

**Vulnerability Category:** `Best Practices`

**Severity:** `LOW` to `MEDIUM`

## Description

This rule flags the use of functions that are considered unsafe or inappropriate for production environments. These are often debugging tools or functions that could have unintended side effects if left in production code.

Common examples of blacklisted functions include:

- **`pdb.set_trace()`:** An interactive Python debugger. If this is called in production, it will halt execution and expose an interactive debugging shell.
- **`print()` statements (sometimes):** Excessive or sensitive information logged via `print` can leak data in production.
- **`eval()` or `exec()`:** (See [CSP-D001](./CSP-D001.md) and [CSP-D002](./CSP-D002.md)) While not always blacklisted, their use can be flagged if they are deemed too risky for the context.
- **Legacy or insecure functions:** Such as `os.tempnam()` ([CSP-D506](./CSP-D506.md)) or `telnetlib.Telnet()` ([CSP-D409](./CSP-D409.md)), which are covered by specific rules but might also be caught here as general blacklisted items.

Leaving such functions in production code can lead to information disclosure, denial of service, or even remote code execution.

## Vulnerable Code Example

```python
import pdb
import os

def process_data(data):
    # Debugger set during development
    pdb.set_trace()

    result = os.system(f"echo {data}") # Example of a potentially dangerous call
    return result

# In production, calling process_data might halt execution and expose the debugger.
```

## Safe Code Example

Remove all debugging statements and blacklisted functions before deploying to production. Ensure that only essential, production-ready code is present.

```python
def process_data(data):
    # Debugger removed for production
    # pdb.set_trace()

    # Use safer alternatives for shell commands if possible,
    # or ensure strict validation if shell=True is unavoidable.
    # For this example, let's assume we use a safer subprocess call or
    # a different, secure method.
    # result = subprocess.run(['echo', data], capture_output=True, text=True)

    # Simplified for example: replace with safe logic.
    print(f"Processing data: {data}")
    return True
```

## How to Suppress a Finding

If a blacklisted function is intentionally used in production for a specific, justified reason (e.g., an auditing tool that needs to log verbose information, or a controlled debugging endpoint in a secure environment), you can suppress the finding with a comment. However, this is rare and should be done with extreme caution.

```python
# This print statement is part of an internal security audit logging mechanism.
# ignore
print(f"Security event logged: {event_details}")
```

Or, for this specific rule:

```python
# ignore: CSP-D704
pdb.set_trace() # Intentionally included for a specific debugging scenario
```
