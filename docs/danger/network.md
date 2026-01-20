# Category 5: Network & HTTP (CSP-D4xx)

Rules in this category detect insecure network configurations, SSRF vulnerabilities, and missing timeouts.

| Rule ID      | Pattern                             | Severity     | Why it's risky              | Safer alternative / Fix               |
| :----------- | :---------------------------------- | :----------- | :-------------------------- | :------------------------------------ |
| **CSP-D401** | `requests.*(verify=False)`          | **HIGH**     | MITM attacks                | Keep `verify=True`                    |
| **CSP-D402** | Unvalidated URLs in network calls   | **CRITICAL** | SSRF (Request forgery)      | Allowlist domains; validate host/port |
| **CSP-D403** | `app.run(debug=True)`               | **HIGH**     | Possible RCE in production  | Set `debug=False`                     |
| **CSP-D404** | Hardcoded bind to `0.0.0.0` or `::` | **MEDIUM**   | Exposes service to external | Bind to `127.0.0.1` locally           |
| **CSP-D405** | Request without timeout             | **MEDIUM**   | Thread/Process exhaustion   | Set `timeout=5.0` (or similar)        |
| **CSP-D406** | `ftplib.*`                          | **MEDIUM**   | Cleartext FTP traffic       | Use SFTP or FTPS                      |
| **CSP-D407** | `HTTPSConnection` without context   | **MEDIUM**   | MITM on legacy Python       | Provide a secure SSL context          |
| **CSP-D408** | `ssl._create_unverified_context`    | **MEDIUM**   | Bypasses SSL verification   | Use default secure context            |
| **CSP-D409** | `telnetlib.*`                       | **MEDIUM**   | Cleartext Telnet traffic    | Use SSH (`paramiko`)                  |
| **CSP-D410** | `urllib.urlopen` (audit schemes)    | **MEDIUM**   | `file://` scheme exploits   | Validate/restrict schemes             |
| **CSP-D411** | `ssl.wrap_socket` (deprecated)      | **MEDIUM**   | Often insecure/deprecated   | Use `SSLContext.wrap_socket`          |

## In-depth: SSRF (CSP-D402)

Server-Side Request Forgery (SSRF) allows an attacker to make the server perform requests to internal or external resources.

### Dangerous Pattern

```python
import requests
url = request.args.get("url")
requests.get(url) # VULNERABLE to SSRF
```

### Safe Alternative

```python
VALID_DOMAINS = ["api.example.com"]
url = request.args.get("url")
if get_domain(url) in VALID_DOMAINS:
    requests.get(url) # SAFE: Validated
```
