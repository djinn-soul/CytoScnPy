# CSP-D506: Insecure Temporary File Creation with `os.tempnam` or `os.tmpnam`

**Vulnerability Category:** `Filesystem`

**Severity:** `MEDIUM`

## Description

This rule flags the use of `os.tempnam()` and `os.tmpnam()`. These functions are insecure for the same reason as `tempfile.mktemp()` (see [CSP-D504](./CSP-D504.md)): they generate a temporary filename but do not create the file, leading to a Time-of-Check to Time-of-Use (TOCTOU) race condition.

An attacker can observe the predictable temporary filename and create a symbolic link at that path before the application opens the file. This can lead to the application overwriting sensitive system files.

Both `os.tempnam()` and `os.tmpnam()` have been deprecated since Python 2.3 and will raise a `RuntimeWarning`. They should never be used.

## Vulnerable Code Example

```python
import os

# os.tempnam() returns a path to a file that does not yet exist.
# This is a race condition waiting to happen.
tmp_path = os.tempnam('/tmp', 'myapp_')

# Attacker can create a symlink:
# ln -s /root/.ssh/authorized_keys /tmp/myapp_...

# The application, thinking it's writing to a temp file,
# will now overwrite the root user's authorized_keys file.
with open(tmp_path, 'w') as f:
    f.write('ssh-rsa AAAA... attacker@key')
```

## Safe Code Example

Always use the modern `tempfile` module to create temporary files and directories. These functions create the file/directory atomically, preventing race conditions.

### Using `NamedTemporaryFile`
This is a convenient, high-level approach that handles cleanup automatically.

```python
import tempfile

# Creates a file that is automatically deleted when the 'with' block exits.
# There is no race condition.
with tempfile.NamedTemporaryFile(mode='w', dir='/tmp', prefix='myapp_') as tmp:
    print(f"Created secure temporary file: {tmp.name}")
    tmp.write('This is safe and will be cleaned up.')
```

### Using `mkstemp()`
This is a lower-level function that returns a file descriptor and path. You are responsible for cleanup.

```python
import tempfile
import os

# Creates the file securely and returns a handle and path.
fd, path = tempfile.mkstemp(dir='/tmp', prefix='myapp_')

try:
    with os.fdopen(fd, 'w') as tmp:
        tmp.write('This is also safe.')
finally:
    # You must remove the file yourself.
    os.remove(path)
```

## How to Suppress a Finding

There is no valid reason to use `os.tempnam` or `os.tmpnam`. These functions are fundamentally insecure and have been deprecated for over a decade. You should always migrate to the `tempfile` module.

If you are analyzing legacy code that cannot be changed, you can suppress the warning, but it is critical to understand that a security risk remains.

```python
# This code is for a legacy system and cannot be altered.
# The security risk is acknowledged.
# ignore
tmp_path = os.tempnam()
```

Or, for this specific rule:

```python
# ignore: CSP-D506
tmp_path = os.tmpnam()
```
