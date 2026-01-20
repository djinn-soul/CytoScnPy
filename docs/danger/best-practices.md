# Category 7: Best Practices (CSP-D7xx)

Rules in this category detect violations of Python best practices that often lead to security issues or regressions in production.

| Rule ID      | Pattern                                            | Severity   | Why it's risky                               | Safer alternative / Fix                     |
| :----------- | :------------------------------------------------- | :--------- | :------------------------------------------- | :------------------------------------------ |
| **CSP-D701** | `assert` used in production code                   | LOW        | Asserts are removed in optimized mode (`-O`) | Use explicit `if ...: raise`                |
| **CSP-D702** | Insecure Imports (`telnetlib`, `ftplib`, etc)      | HIGH / LOW | Use of deprecated/insecure libraries         | Use modern replacements (`requests`, `ssh`) |
| **CSP-D703** | `Jinja2 Environment(autoescape=False)`             | HIGH       | Risk of XSS if content is not escaped        | Set `autoescape=True`                       |
| **CSP-D704** | Blacklisted function calls (e.g., `pdb.set_trace`) | LOW / MED  | Debugging leftovers in production            | Remove debug code                           |

## In-depth: Asserts in Production (CSP-D701)

The `assert` statement is intended for internal self-checks during development. Python's bytecode compiler removes all assert statements when compiling with optimization enabled (`python -O`). Relying on assert for security logic (e.g., `assert user.is_admin`) leads to bypasses in production.

### Dangerous Pattern

```python
def delete_user(user):
    assert user.is_admin, "Not allowed"
    # ... delete logic ...
```

### Safe Alternative

```python
def delete_user(user):
    if not user.is_admin:
        raise PermissionError("Not allowed")
    # ... delete logic ...
```

## In-depth: Insecure Imports (CSP-D702)

Certain standard library modules are considered insecure or obsolete.

- `telnetlib`: Telnet transmits data in cleartext. Use SSH (e.g., `paramiko` or `fabric`).
- `ftplib`: FTP transmits credentials in cleartext. Use SFTP or FTPS.
- `xml.etree`, `xml.sax`: Vulnerable to XXE (XML External Entity) attacks. Use `defusedxml`.
- `pickle`, `marshal`: Insecure deserialization. Use `json` or `hmac` signatures.

### Dangerous Pattern

```python
import telnetlib
tn = telnetlib.Telnet("host") # VULNERABLE: Cleartext traffic
```

### Safe Alternative

```python
# Use a library that supports SSH/SCP/SFTP
import paramiko
client = paramiko.SSHClient()
# ...
```
