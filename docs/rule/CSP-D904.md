# CSP-D904: Log Injection

**Vulnerability Category:** `Privacy`

**Severity:** `MEDIUM`

## Description

This rule flags dynamic string construction in logging calls, such as f-strings, string concatenation, or percent-formatting passed directly to `logging` or common logger variables.

Log injection occurs when untrusted data containing newline or carriage-return characters is written into logs without neutralization. An attacker can forge extra log entries, hide activity, corrupt structured logs, or mislead automated log processing.

## Vulnerable Code Example

```python
import logging

def login(request):
    username = request.GET["username"]
    logging.info(f"Login attempt for {username}")
```

If `username` contains `\n` or `\r`, the resulting log output can include attacker-controlled fake entries.

## Safe Code Example

```python
import logging

def clean_for_log(value):
    return str(value).replace("\r", "\\r").replace("\n", "\\n")

def login(request):
    username = clean_for_log(request.GET["username"])
    logging.info("Login attempt for %s", username)
```

Keep the log message template static and neutralize control characters in untrusted values before logging them.

## Notes

This rule is about log integrity, not ordinary status text. Literal messages such as `logging.info("Processing started")` are safe and should not be flagged.

## How to Suppress a Finding

Suppress only when dynamic values are already sanitized for log output or are guaranteed not to contain attacker-controlled content.

```python
# username has CR/LF escaped by clean_for_log().
# ignore: CSP-D904
logging.info("Login attempt for " + username)
```
