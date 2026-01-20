# Category 1: Code Execution & Unsafe calls (CSP-D0xx)

Rules in this category detect patterns that can lead to arbitrary code execution or command injection. These are the highest-risk findings.

| Rule ID      | Pattern                                      | Severity     | Why it's risky                 | Safer alternative / Fix                        |
| :----------- | :------------------------------------------- | :----------- | :----------------------------- | :--------------------------------------------- |
| **CSP-D001** | `eval(...)`                                  | HIGH         | Arbitrary code execution       | Use `ast.literal_eval` or a dedicated parser   |
| **CSP-D002** | `exec(...)`                                  | HIGH         | Arbitrary code execution       | Remove or use explicit dispatch                |
| **CSP-D003** | `os.system(...)`, `subprocess.*(shell=True)` | **CRITICAL** | Command injection              | `subprocess.run([cmd, ...])`; strict allowlist |
| **CSP-D004** | `asyncio.create_subprocess_shell(...)`       | **CRITICAL** | Async command injection        | Use `create_subprocess_exec` with list args    |
| **CSP-D005** | `input(...)`                                 | HIGH         | Unsafe in Py2 (acts like eval) | Use `raw_input()` (Py2) or validate in Py3     |

## In-depth: Async & Legacy Shell Injection (CSP-D004)

Traditional shell execution functions and modern async variants are equally dangerous when given untrusted input.

### Dangerous Pattern

```python
import asyncio
import os

# Async shell injection
await asyncio.create_subprocess_shell(f"ls {user_input}")

# Legacy popen injection (This is actually usually CSP-D003 or CSP-D004 depending on rule logic, D004 for async/legacy separation if applicable)
# Wait, META_SUBPROCESS checks popen too. But META_ASYNC_SUBPROCESS checks async.
# I will keep the example but update the ID.
```

### Safe Alternative

```python
import asyncio
import os

# Async shell injection
await asyncio.create_subprocess_shell(f"ls {user_input}")

# Legacy popen injection
os.popen(f"cat {user_input}")
```

### Safe Alternative

```python
import asyncio
import subprocess

# Safe async execution
await asyncio.create_subprocess_exec("ls", user_input)

# Safe synchronous execution
subprocess.run(["cat", user_input])
```

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
