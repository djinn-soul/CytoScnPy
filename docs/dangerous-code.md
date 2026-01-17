# Dangerous Code Rules Reference

Security rules detected by the `--danger` flag, organized by category.

---

## Category 1: Code Execution & Unsafe calls (CSP-D0xx)

| Rule ID      | Pattern                                       | Why it's risky                 | Safer alternative / Fix                        |
| :----------- | :-------------------------------------------- | :----------------------------- | :--------------------------------------------- |
| **CSP-D001** | `eval(...)`                                   | Arbitrary code execution       | Use `ast.literal_eval` or a dedicated parser   |
| **CSP-D002** | `exec(...)`                                   | Arbitrary code execution       | Remove or use explicit dispatch                |
| **CSP-D003** | `os.system(...)` / `subprocess.*(shell=True)` | Command injection              | `subprocess.run([cmd, ...])`; strict allowlist |
| **CSP-D004** | Insecure Imports                              | (See Category 8)               | Use modern/secure alternatives                 |
| **CSP-D005** | `telnetlib.*` calls                           | Telnet is unencrypted          | Use SSH / Cryptography                         |
| **CSP-D006** | `ftplib.*` calls                              | FTP is unencrypted             | Use SFTP / SCP                                 |
| **CSP-D007** | `input()`                                     | Unsafe in Py2 (acts like eval) | Use `raw_input()` (Py2) or `input()` (Py3)     |

---

## Category 2: Injection & Logic Attacks (CSP-D1xx)

| Rule ID      | Pattern                                 | Why it's risky                | Safer alternative / Fix                  |
| :----------- | :-------------------------------------- | :---------------------------- | :--------------------------------------- |
| **CSP-D101** | `cursor.execute` (f-string/concat)      | SQL injection (cursor)        | Use parameterized queries (`?`, `%s`)    |
| **CSP-D102** | `sqlalchemy.text`, `read_sql` (dynamic) | SQL injection (raw)           | Use bound parameters / ORM builders      |
| **CSP-D103** | Flask/Jinja dynamic templates           | XSS (Cross-site scripting)    | Use static templates; escape content     |
| **CSP-D104** | `xml.etree`, `minidom`, `sax`, `lxml`   | XXE / DoS                     | Use `defusedxml`                         |
| **CSP-D105** | `assert` used in production             | Asserts can be optimized away | Use explicit `if ...: raise`             |
| **CSP-D106** | `Jinja2 Environment(autoescape=False)`  | XSS vulnerability             | Set `autoescape=True`                    |
| **CSP-D107** | `django.utils.safestring.mark_safe`     | XSS bypass                    | Avoid unless content is strictly trusted |

---

## Category 3: Deserialization (CSP-D2xx)

| Rule ID      |             Pattern             | Why it's risky            | Safer alternative / Fix           |
| :----------- | :-----------------------------: | :------------------------ | :-------------------------------- |
| **CSP-D201** | `pickle.load`, `dill`, `shelve` | Arbitrary code execution  | Use JSON, msgpack, or signed data |
| **CSP-D202** |   `yaml.load` (no SafeLoader)   | Arbitrary code execution  | `yaml.safe_load(...)`             |
| **CSP-D203** |     `marshal.load`/`loads`      | Unsafe for untrusted data | Use secure serialization          |

---

## Category 4: Cryptography & Randomness (CSP-D3xx)

| Rule ID      | Pattern                                | Why it's risky              | Safer alternative / Fix       |
| :----------- | :------------------------------------- | :-------------------------- | :---------------------------- |
| **CSP-D301** | `hashlib.md5`, `hashlib.new('md5')`    | Collision-prone weak hash   | Use SHA-256 or SHA-3          |
| **CSP-D302** | `hashlib.sha1`, `hashlib.new('sha1')`  | Weak cryptographic hash     | Use SHA-256 or SHA-3          |
| **CSP-D304** | Insecure Ciphers (ARC4, DES, Blowfish) | Successfully broken ciphers | Use AES (GCM/CTR)             |
| **CSP-D305** | Insecure Cipher Mode (ECB)             | Pattern leakage             | Use CBC or GCM                |
| **CSP-D311** | `random.*` (Standard PRNG)             | Predictable for crypto use  | Use `secrets` or `os.urandom` |

---

## Category 5: Network & HTTP (CSP-D4xx)

| Rule ID      | Pattern                             | Why it's risky                     | Safer alternative / Fix               |
| :----------- | :---------------------------------- | :--------------------------------- | :------------------------------------ |
| **CSP-D401** | `requests.*(verify=False)`          | MITM attacks                       | Keep `verify=True`                    |
| **CSP-D402** | Unvalidated URLs in network calls   | SSRF (Request forgery)             | Allowlist domains; validate host/port |
| **CSP-D403** | `app.run(debug=True)`               | Possible RCE in production         | Set `debug=False`                     |
| **CSP-D404** | Hardcoded bind to `0.0.0.0` or `::` | Exposes service to external        | Bind to `127.0.0.1` locally           |
| **CSP-D405** | Request without timeout             | Thread/Process exhaustion          | Set `timeout=5.0` (or similar)        |
| **CSP-D406** | `urllib` audit                      | File scheme vulnerabilities        | Validate URL schemes (https only)     |
| **CSP-D407** | `ssl._create_unverified_context`    | Certificate bypass                 | Use default secure context            |
| **CSP-D408** | `HTTPSConnection` without context   | Insecure defaults in some versions | Pass explicit SSL context             |
| **CSP-D409** | `ssl.wrap_socket`                   | Deprecated, often insecure         | Use `ssl.create_default_context()`    |

---

## Category 6: File Operations & Temporary Data (CSP-D5xx)

| Rule ID      | Pattern                             |      Why it's risky       | Safer alternative / Fix            |
| :----------- | :---------------------------------- | :-----------------------: | :--------------------------------- |
| **CSP-D501** | Dynamic path in `open`/`os.path`    |      Path traversal       | Use `Path.resolve`, check base dir |
| **CSP-D502** | `tarfile.extractall` without filter | Path traversal / Zip Slip | Use `filter='data'` (Py 3.12+)     |
| **CSP-D503** | `zipfile.ZipFile.extractall`        | Path traversal / Zip Slip | Validate member filenames          |
| **CSP-D504** | `tempfile.mktemp`                   |  Race condition (TOCTOU)  | Use `tempfile.mkstemp`             |
| **CSP-D505** | `os.chmod` with `stat.S_IWOTH`      |    World-writable file    | Use stricter permissions (0o600)   |
| **CSP-D506** | `os.tempnam`/`tmpnam`               |      Symlink attacks      | Use `tempfile` module              |

---

## Category 7: Type Safety (CSP-D6xx)

| Rule ID      | Pattern                  | Why it's risky                | Safer alternative / Fix          |
| :----------- | :----------------------- | :---------------------------- | :------------------------------- |
| **CSP-D601** | Type-based method misuse | Logic errors / Type confusion | Use static typing and validation |

---

## Category 8: Insecure Library Imports (CSP-D004)

Detection of libraries with inherent security risks.

| Pattern                           | Risk                       | Safer alternative    |
| :-------------------------------- | :------------------------- | :------------------- |
| `telnetlib`, `ftplib`             | Unencrypted communication  | SSH, SFTP            |
| `pickle`, `dill`, `shelve`        | Deserialization RCE        | JSON, msgpack        |
| `xml.etree`, `xml.sax`, `xml.dom` | XXE / DoS                  | `defusedxml`         |
| `xmlrpc`                          | Remote-enabled XML attacks | `defusedxml.xmlrpc`  |
| `lxml`                            | High risk XXE              | `defusedxml.lxml`    |
| `Crypto.*` (pycrypto)             | Unmaintained, known CVEs   | `cryptography`       |
| `pyghmi`                          | IPMI vulnerabilities       | Secure OOB protocols |
| `wsgiref.handlers.CGIHandler`     | httpoxy vulnerability      | Modern WSGI servers  |

---

## Category 9: Modern Python Patterns (CSP-D9xx)

Security rules for modern Python features and ML/AI workflows introduced in 2025/2026.

| Rule ID      | Pattern                                         | Why it's risky                      | Safer alternative / Fix                        |
| :----------- | :---------------------------------------------- | :---------------------------------- | :--------------------------------------------- |
| **CSP-D901** | `asyncio.create_subprocess_shell(dynamic)`      | Async command injection             | Use `create_subprocess_exec` with list args    |
| **CSP-D901** | `os.popen(dynamic)`                             | Legacy command injection            | Use `subprocess.run` with list args            |
| **CSP-D901** | `pty.spawn(dynamic)`                            | PTY command injection               | Validate/allowlist commands                    |
| **CSP-D902** | `torch.load()` without `weights_only=True`      | Arbitrary code execution via pickle | Use `weights_only=True` or `torch.safe_load()` |
| **CSP-D902** | `joblib.load()`                                 | Arbitrary code execution            | Ensure trusted model sources only              |
| **CSP-D902** | `keras.models.load_model()` without `safe_mode` | Lambda layers can execute code      | Use `safe_mode=True`                           |
| **CSP-D903** | Logging sensitive variables                     | Data leakage in logs                | Redact passwords, tokens, API keys             |

---

## Category 10: Framework Security (CSP-D9xx)

| Rule ID      | Pattern                                         | Why it's risky                      | Safer alternative / Fix                        |
| :----------- | :---------------------------------------------- | :---------------------------------- | :--------------------------------------------- |
| **CSP-D901** | `asyncio.create_subprocess_shell(dynamic)`      | Async command injection             | Use `create_subprocess_exec` with list args    |
| **CSP-D901** | `os.popen(dynamic)`                             | Legacy command injection            | Use `subprocess.run` with list args            |
| **CSP-D901** | `pty.spawn(dynamic)`                            | PTY command injection               | Validate/allowlist commands                    |
| **CSP-D902** | `torch.load()` without `weights_only=True`      | Arbitrary code execution via pickle | Use `weights_only=True` or `torch.safe_load()` |
| **CSP-D902** | `joblib.load()`                                 | Arbitrary code execution            | Ensure trusted model sources only              |
| **CSP-D902** | `keras.models.load_model()` without `safe_mode` | Lambda layers can execute code      | Use `safe_mode=True`                           |
| **CSP-D903** | Logging sensitive variables                     | Data leakage in logs                | Redact passwords, tokens, API keys             |
| **CSP-D904** | Hardcoded `SECRET_KEY`                          | Key exposure in Django              | Store in environment variables                 |

---
