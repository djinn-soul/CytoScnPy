# Dangerous Code

This document outlines potential security vulnerabilities and dangerous coding practices that CytoScnPy can detect. Each rule is categorized and includes a description, severity, and recommendations for remediation.

## Code Execution

These rules identify code that can lead to arbitrary code execution.

| Rule ID                      | Description                                  | Severity     | Impact                        | Recommendation                                 |
| :--------------------------- | :------------------------------------------- | :----------- | :---------------------------- | :--------------------------------------------- |
| [CSP-D001](rule/CSP-D001.md) | `eval(...)`                                  | HIGH         | Arbitrary code execution      | Use `ast.literal_eval` or a dedicated parser   |
| [CSP-D002](rule/CSP-D002.md) | `exec(...)`                                  | HIGH         | Arbitrary code execution      | Remove or use explicit dispatch                |
| [CSP-D003](rule/CSP-D003.md) | `os.system(...)`, `subprocess.*(shell=True)` | **CRITICAL** | Command injection             | `subprocess.run([cmd, ...])`; strict allowlist |
| [CSP-D004](rule/CSP-D004.md) | `asyncio.create_subprocess_shell(...)`       | **CRITICAL** | Async command injection       | Use `create_subprocess_exec` with list args    |
| [CSP-D005](rule/CSP-D005.md) | `input(...)`                                 | HIGH         | ACE in legacy Python contexts | Validate input; safe in Python 3               |

---

## Injection

These rules detect various forms of injection vulnerabilities.

| Rule ID                      | Description                             | Severity     | Impact                     | Recommendation                           |
| :--------------------------- | :-------------------------------------- | :----------- | :------------------------- | :--------------------------------------- |
| [CSP-D101](rule/CSP-D101.md) | `cursor.execute` (f-string/concat)      | **CRITICAL** | SQL injection (cursor)     | Use parameterized queries (`?`, `%s`)    |
| [CSP-D102](rule/CSP-D102.md) | `sqlalchemy.text`, `read_sql` (dynamic) | **CRITICAL** | SQL injection (raw)        | Use bound parameters / ORM builders      |
| [CSP-D103](rule/CSP-D103.md) | Flask/Jinja dynamic templates           | **CRITICAL** | XSS (Cross-site scripting) | Use static templates; escape content     |
| [CSP-D104](rule/CSP-D104.md) | `xml.etree`, `minidom`, `sax`, `lxml`   | HIGH / MED   | XXE / DoS                  | Use `defusedxml`                         |
| [CSP-D105](rule/CSP-D105.md) | `django.utils.safestring.mark_safe`     | **MEDIUM**   | XSS bypass                 | Avoid unless content is strictly trusted |

---

## Deserialization

These rules identify insecure deserialization practices.

| Rule ID                      | Description                                                    | Severity     | Impact                   | Recommendation                    |
| :--------------------------- | :------------------------------------------------------------- | :----------- | :----------------------- | :-------------------------------- |
| [CSP-D201](rule/CSP-D201.md) | `pickle`, `dill`, `shelve`, `jsonpickle`, `pandas.read_pickle` | **CRITICAL** | Arbitrary code execution | Use JSON, msgpack, or signed data |
| [CSP-D202](rule/CSP-D202.md) | `yaml.load` (no SafeLoader)                                    | **HIGH**     | Arbitrary code execution | `yaml.safe_load(...)`             |
| [CSP-D203](rule/CSP-D203.md) | `marshal.load`/`loads`                                         | **MEDIUM**   | Arbitrary code execution | Use JSON or signed data           |
| [CSP-D204](rule/CSP-D204.md) | `torch.load`, `keras.load_model`, `joblib.load`                | **CRITICAL** | ACE via embedded pickle  | Use `weights_only=True` (torch)   |

---

## Cryptography

These rules highlight the misuse of cryptographic primitives.

| Rule ID                      | Description                        | Severity   | Impact                     | Recommendation                |
| :--------------------------- | :--------------------------------- | :--------- | :------------------------- | :---------------------------- |
| [CSP-D301](rule/CSP-D301.md) | Weak hashing (MD5, etc.)           | **MEDIUM** | Collision-prone weak hash  | Use SHA-256 or SHA-3          |
| [CSP-D302](rule/CSP-D302.md) | Weak hashing (SHA-1)               | **MEDIUM** | Collision-prone weak hash  | Use SHA-256 or SHA-3          |
| [CSP-D304](rule/CSP-D304.md) | Insecure ciphers (DES, ARC4, etc.) | **HIGH**   | Process/Data compromise    | Use AES                       |
| [CSP-D305](rule/CSP-D305.md) | Insecure cipher modes (ECB)        | **MEDIUM** | Pattern leakage in cipher  | Use CBC or GCM                |
| [CSP-D311](rule/CSP-D311.md) | `random.*` (Standard PRNG)         | LOW        | Predictable for crypto use | Use `secrets` or `os.urandom` |

---

## Network

These rules relate to insecure network communication practices.

| Rule ID                      | Description                         | Severity     | Impact                      | Recommendation                        |
| :--------------------------- | :---------------------------------- | :----------- | :-------------------------- | :------------------------------------ |
| [CSP-D401](rule/CSP-D401.md) | `requests.*(verify=False)`          | **HIGH**     | MITM attacks                | Keep `verify=True`                    |
| [CSP-D402](rule/CSP-D402.md) | Unvalidated URLs in network calls   | **CRITICAL** | SSRF (Request forgery)      | Allowlist domains; validate host/port |
| [CSP-D403](rule/CSP-D403.md) | `app.run(debug=True)`               | **HIGH**     | Possible RCE in production  | Set `debug=False`                     |
| [CSP-D404](rule/CSP-D404.md) | Hardcoded bind to `0.0.0.0` or `::` | **MEDIUM**   | Exposes service to external | Bind to `127.0.0.1` locally           |
| [CSP-D405](rule/CSP-D405.md) | Request without timeout             | **MEDIUM**   | Thread/Process exhaustion   | Set `timeout=5.0` (or similar)        |
| [CSP-D406](rule/CSP-D406.md) | `ftplib.*`                          | **MEDIUM**   | Cleartext FTP traffic       | Use SFTP or FTPS                      |
| [CSP-D407](rule/CSP-D407.md) | `HTTPSConnection` without context   | **MEDIUM**   | MITM on legacy Python       | Provide a secure SSL context          |
| [CSP-D408](rule/CSP-D408.md) | `ssl._create_unverified_context`    | **MEDIUM**   | Bypasses SSL verification   | Use default secure context            |
| [CSP-D409](rule/CSP-D409.md) | `telnetlib.*`                       | **MEDIUM**   | Cleartext Telnet traffic    | Use SSH (`paramiko`)                  |
| [CSP-D410](rule/CSP-D410.md) | `urllib.urlopen` (audit schemes)    | **MEDIUM**   | `file://` scheme exploits   | Validate/restrict schemes             |
| [CSP-D411](rule/CSP-D411.md) | `ssl.wrap_socket` (deprecated)      | **MEDIUM**   | Often insecure/deprecated   | Use `SSLContext.wrap_socket`          |

---

## Filesystem

These rules relate to insecure file system operations.

| Rule ID                      | Description                         | Severity   | Impact                    | Recommendation                     |
| :--------------------------- | :---------------------------------- | :--------- | :------------------------ | :--------------------------------- |
| [CSP-D501](rule/CSP-D501.md) | Dynamic path in `open`/`os.path`    | **HIGH**   | Path traversal            | Use `Path.resolve`, check base dir |
| [CSP-D502](rule/CSP-D502.md) | `tarfile.extractall` without filter | **HIGH**   | Path traversal / Zip Slip | Use `filter='data'` (Py 3.12+)     |
| [CSP-D503](rule/CSP-D503.md) | `zipfile.ZipFile.extractall`        | **HIGH**   | Path traversal / Zip Slip | Validate member filenames          |
| [CSP-D504](rule/CSP-D504.md) | `tempfile.mktemp`                   | **HIGH**   | Race condition (TOCTOU)   | Use `tempfile.mkstemp`             |
| [CSP-D505](rule/CSP-D505.md) | `os.chmod` with `stat.S_IWOTH`      | **HIGH**   | World-writable file       | Use stricter permissions (0o600)   |
| [CSP-D506](rule/CSP-D506.md) | `os.tempnam`/`tmpnam`               | **MEDIUM** | Symlink attacks           | Use `tempfile` module              |

---

## Type Safety

These rules address potential issues related to type handling.

| Rule ID                      | Description              | Severity | Impact                        | Recommendation                   |
| :--------------------------- | :----------------------- | :------- | :---------------------------- | :------------------------------- |
| [CSP-D601](rule/CSP-D601.md) | Type-based method misuse | **HIGH** | Logic errors / Type confusion | Use static typing and validation |

---

## Best Practices

These rules highlight deviations from recommended secure coding practices.

| Rule ID                      | Description                                        | Severity   | Impact                                       | Recommendation                              |
| :--------------------------- | :------------------------------------------------- | :--------- | :------------------------------------------- | :------------------------------------------ |
| [CSP-D701](rule/CSP-D701.md) | `assert` used in production code                   | LOW        | Asserts are removed in optimized mode (`-O`) | Use explicit `if ...: raise`                |
| [CSP-D702](rule/CSP-D702.md) | Insecure Imports (`telnetlib`, `ftplib`, etc)      | HIGH / LOW | Use of deprecated/insecure libraries         | Use modern replacements (`requests`, `ssh`) |
| [CSP-D703](rule/CSP-D703.md) | `Jinja2 Environment(autoescape=False)`             | HIGH       | Risk of XSS if content is not escaped        | Set `autoescape=True`                       |
| [CSP-D704](rule/CSP-D704.md) | Blacklisted function calls (e.g., `pdb.set_trace`) | LOW / MED  | Debugging leftovers in production            | Remove debug code                           |

---

## Open Redirect

This category covers vulnerabilities related to insecure redirection.

| Rule ID                      | Description   | Severity | Impact                              | Recommendation                 |
| :--------------------------- | :------------ | :------- | :---------------------------------- | :----------------------------- |
| [CSP-D801](rule/CSP-D801.md) | Open Redirect | **HIGH** | User redirection to malicious sites | Validate redirect URLs/domains |

---

## Privacy

These rules address potential privacy violations.

| Rule ID                      | Description                 | Severity     | Impact                 | Recommendation                     |
| :--------------------------- | :-------------------------- | :----------- | :--------------------- | :--------------------------------- |
| [CSP-D901](rule/CSP-D901.md) | Logging sensitive variables | **MEDIUM**   | Data leakage in logs   | Redact passwords, tokens, API keys |
| [CSP-D902](rule/CSP-D902.md) | Hardcoded `SECRET_KEY`      | **CRITICAL** | Key exposure in Django | Store in environment variables     |

---

## Generic

This is a catch-all category for general vulnerabilities.

| Rule ID                      | Description                      | Severity     | Impact                         | Recommendation                                 |
| :--------------------------- | :------------------------------- | :----------- | :----------------------------- | :--------------------------------------------- |
| [CSP-X001](rule/CSP-X001.md) | Generic XSS (detected via taint) | **CRITICAL** | Potential for script injection | Sanitize/encode output, use templating engines |
