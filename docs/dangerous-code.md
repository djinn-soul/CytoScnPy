# Dangerous Code Rules Reference

Security rules detected by the `--danger` flag, organized by category.

---

## Category 1: Code Execution (CSP-D0xx) - Highest Risk

| Rule ID  | Pattern                                                              | Why it's risky           | Safer alternative / Fix                                    |
| -------- | -------------------------------------------------------------------- | ------------------------ | ---------------------------------------------------------- |
| CSP-D001 | `eval(...)`                                                          | Arbitrary code execution | Use `ast.literal_eval` or a dedicated parser               |
| CSP-D002 | `exec(...)`                                                          | Arbitrary code execution | Remove or use explicit dispatch                            |
| CSP-D003 | `os.system(<tainted>)` or `subprocess.*(..., shell=True, <tainted>)` | Command injection        | `subprocess.run([cmd, ...], check=True)`; strict allowlist |

---

## Category 2: Injection Attacks (CSP-D1xx)

| Rule ID  | Pattern                                                                           | Why it's risky             | Safer alternative / Fix                                         |
| -------- | --------------------------------------------------------------------------------- | -------------------------- | --------------------------------------------------------------- |
| CSP-D101 | `cursor.execute` / `executemany` with f-string or string-built SQL                | SQL injection (cursor)     | Use parameterized queries (`WHERE name = ?` with params)        |
| CSP-D102 | `sqlalchemy.text(...)`, `pandas.read_sql*`, `*.objects.raw(...)` with dynamic SQL | SQL injection (raw-api)    | Use bound parameters/ORM query builders                         |
| CSP-D103 | `flask.render_template_string(...)` / `jinja2.Markup(...)` with dynamic content   | XSS (cross-site scripting) | Use `render_template()` with separate files; escape input       |
| CSP-D104 | `xml.etree.ElementTree`, `xml.dom.minidom`, `xml.sax`, `lxml.etree` parsing       | XXE / Billion Laughs DoS   | Use `defusedxml` library; for lxml use `resolve_entities=False` |

---

## Category 3: Deserialization (CSP-D2xx)

| Rule ID  | Pattern                                  | Why it's risky                  | Safer alternative / Fix                                      |
| -------- | ---------------------------------------- | ------------------------------- | ------------------------------------------------------------ |
| CSP-D201 | `pickle.load(...)` / `pickle.loads(...)` | Untrusted deserialization       | Avoid for untrusted data; only load trusted pickle files     |
| CSP-D202 | `yaml.load(...)` (no SafeLoader)         | Can construct arbitrary objects | `yaml.safe_load(...)` or `yaml.load(..., Loader=SafeLoader)` |

---

## Category 4: Cryptography (CSP-D3xx)

| Rule ID  | Pattern             | Why it's risky      | Safer alternative / Fix              |
| -------- | ------------------- | ------------------- | ------------------------------------ |
| CSP-D301 | `hashlib.md5(...)`  | Weak hash algorithm | `hashlib.sha256(...)` or HMAC-SHA256 |
| CSP-D302 | `hashlib.sha1(...)` | Weak hash algorithm | `hashlib.sha256(...)` or HMAC-SHA256 |

---

## Category 5: Network/HTTP (CSP-D4xx)

| Rule ID  | Pattern                                                                    | Why it's risky                     | Safer alternative / Fix                          |
| -------- | -------------------------------------------------------------------------- | ---------------------------------- | ------------------------------------------------ |
| CSP-D401 | `requests.*(verify=False)`                                                 | Disables TLS verification          | Keep default `verify=True` or set CA bundle path |
| CSP-D402 | `requests`/`httpx.*(url)` / `urllib.request.urlopen(url)` with dynamic URL | SSRF (server-side request forgery) | Allowlist domains; validate/sanitize URLs        |

---

## Category 6: File Operations (CSP-D5xx)

| Rule ID  | Pattern                                                             | Why it's risky            | Safer alternative / Fix                              |
| -------- | ------------------------------------------------------------------- | ------------------------- | ---------------------------------------------------- |
| CSP-D501 | `open(path)` / `os.path.(path)` / `shutil.(path)` with dynamic path | Path traversal            | Join to fixed base, `resolve()`, enforce containment |
| CSP-D502 | `tarfile.extractall(...)` without `filter` parameter                | Zip Slip / Path traversal | Use `filter='data'` or `filter='tar'` (Python 3.12+) |
| CSP-D503 | `zipfile.ZipFile.extractall(...)` without path validation           | Zip Slip / Path traversal | Check `ZipInfo.filename` for `..` and absolute paths |

---

## Category 7: Type Safety (CSP-D6xx)

| Rule ID  | Pattern                               | Why it's risky      | Safer alternative / Fix                 |
| -------- | ------------------------------------- | ------------------- | --------------------------------------- |
| CSP-D601 | Method misuse based on inferred types | Type confusion bugs | Use type hints and static type checkers |
