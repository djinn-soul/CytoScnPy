# Category 1: Code Execution & Unsafe calls (CSP-D0xx)

Rules in this category detect patterns that can lead to arbitrary code execution or command injection. These are the highest-risk findings.

| Rule ID      | Pattern                                       | Severity     | Why it's risky                 | Safer alternative / Fix                        |
| :----------- | :-------------------------------------------- | :----------- | :----------------------------- | :--------------------------------------------- |
| **CSP-D001** | `eval(...)`                                   | HIGH         | Arbitrary code execution       | Use `ast.literal_eval` or a dedicated parser   |
| **CSP-D002** | `exec(...)`                                   | HIGH         | Arbitrary code execution       | Remove or use explicit dispatch                |
| **CSP-D003** | `os.system(...)` / `subprocess.*(shell=True)` | **CRITICAL** | Command injection              | `subprocess.run([cmd, ...])`; strict allowlist |
| **CSP-D004** | Insecure Imports (`telnetlib`, `ftplib`, etc) | HIGH / LOW   | Use of inherently unsafe libs  | Use modern/secure alternatives                 |
| **CSP-D007** | `input()`                                     | HIGH         | Unsafe in Py2 (acts like eval) | Use `raw_input()` (Py2) or validate in Py3     |

## In-depth: Command Injection (CSP-D003)

Command injection occurs when an application executes a shell command but does not properly validate or sanitize the arguments.

### Dangerous Pattern

```python
import subprocess
user_input = "ls; rm -rf /"
subprocess.run(f"ls {user_input}", shell=True)
```

### Safe Alternative

```python
import subprocess
user_input = "filename.txt"
subprocess.run(["ls", user_input]) # shell=False is default and safer
```
