# Category 6: File Operations & Temporary Data (CSP-D5xx)

Rules in this category detect path traversal vulnerabilities, insecure archive extraction, and bad file permissions.

| Rule ID      | Pattern                             | Severity   | Why it's risky            | Safer alternative / Fix            |
| :----------- | :---------------------------------- | :--------- | :------------------------ | :--------------------------------- |
| **CSP-D501** | Dynamic path in `open`/`os.path`    | **HIGH**   | Path traversal            | Use `Path.resolve`, check base dir |
| **CSP-D502** | `tarfile.extractall` without filter | **HIGH**   | Path traversal / Zip Slip | Use `filter='data'` (Py 3.12+)     |
| **CSP-D503** | `zipfile.ZipFile.extractall`        | **HIGH**   | Path traversal / Zip Slip | Validate member filenames          |
| **CSP-D504** | `tempfile.mktemp`                   | **HIGH**   | Race condition (TOCTOU)   | Use `tempfile.mkstemp`             |
| **CSP-D505** | `os.chmod` with `stat.S_IWOTH`      | **HIGH**   | World-writable file       | Use stricter permissions (0o600)   |
| **CSP-D506** | `os.tempnam`/`tmpnam`               | **MEDIUM** | Symlink attacks           | Use `tempfile` module              |

## In-depth: Path Traversal (CSP-D501)

Path traversal allows an attacker to access files outside the intended directory.

### Dangerous Pattern

```python
filename = request.args.get("file")
with open(f"uploads/{filename}", "rb") as f: # VULNERABLE to ../../etc/passwd
    data = f.read()
```

### Safe Alternative

```python
import os
filename = request.args.get("file")
base_dir = os.path.abspath("uploads")
full_path = os.path.abspath(os.path.join(base_dir, filename))
if full_path.startswith(base_dir):
    with open(full_path, "rb") as f: # SAFE: Boundary checked
        data = f.read()
```
