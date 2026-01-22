# CSP-D003: Command Injection

**Vulnerability Category:** `Code Execution`

**Severity:** `CRITICAL`

## Description

Command injection vulnerabilities are severe security flaws that allow an attacker to execute arbitrary commands on the host operating system. This vulnerability arises when an application passes untrusted user input to a system shell. Functions like `os.system()` and `subprocess.run()` with `shell=True` are common sources of command injection vulnerabilities.

An attacker can inject shell metacharacters (e.g., `;`, `&&`, `||`, `|`, `` ` ``, `$(...)`) to chain commands and take control of the server.

## Vulnerable Code Example

```python
import os
import subprocess

filename = input("Enter the filename to display: ")

# Vulnerable to command injection
# An attacker could enter: "my_file.txt; rm -rf /"
os.system(f"cat {filename}")

# Also vulnerable
subprocess.run(f"cat {filename}", shell=True)
```

In both `os.system` and `subprocess.run` with `shell=True`, the string is passed to the system's shell, which interprets the user's input as part of the command.

## Safe Code Example

To prevent command injection, avoid using `shell=True` and pass command arguments as a list. This ensures that user input is treated as a single argument and not interpreted by the shell.

```python
import subprocess
import shlex

filename = input("Enter the filename to display: ")

# The command and its arguments are passed as a list
# The user input is treated as a single, safe argument
try:
    # Passing as a list avoids the shell entirely and is safe.
    subprocess.run(["cat", filename], check=True)
except FileNotFoundError:
    print("Error: cat command not found.")
except subprocess.CalledProcessError:
    print(f"Error: Could not display file {filename}.")

```

By passing the arguments as a list, the operating system is responsible for handling the arguments safely, preventing the shell from interpreting them.

## How to Suppress a Finding

Command injection is a critical vulnerability. Suppressing this finding is strongly discouraged. If you must, and have validated that the input is safe, you can use a suppression comment.

```python
# ignore
os.system(f"cat {validated_filename}")
```

Or, for this specific rule:

```python
# ignore: CSP-D003
subprocess.run(f"cat {validated_filename}", shell=True)
```
