# CSP-D504: Insecure Temporary File Creation with `tempfile.mktemp()`

**Vulnerability Category:** `Filesystem`

**Severity:** `HIGH`

## Description

This rule flags the use of `tempfile.mktemp()`, which is an insecure function for creating temporary files. This function generates a unique temporary filename but **does not** create the file. This creates a race condition known as a Time-of-Check to Time-of-Use (TOCTOU) vulnerability.

The vulnerability occurs in the small window of time between when `mktemp()` returns the filename and when your application actually creates and opens the file. An attacker can create a file (or more dangerously, a symbolic link) at that exact path.

If an attacker creates a symbolic link to a sensitive file (e.g., `/etc/passwd`), when the application later opens the temporary filename and writes to it, it will actually be writing to the sensitive file, potentially corrupting it or leading to a denial of service.

The `tempfile.mktemp()` function is so dangerous that it was deprecated in Python 2.3 and will raise a `RuntimeWarning`.

## Vulnerable Code Example

```python
import tempfile
import os

# 1. mktemp() returns a filename that does not exist yet.
#    e.g., '/tmp/tmp123abc'
filename = tempfile.mktemp()

# --- RACE CONDITION WINDOW ---
# An attacker could create a symbolic link at this exact moment:
# ln -s /etc/shadow /tmp/tmp123abc

# 2. The application opens the file, but it's now a symlink to a sensitive file.
with open(filename, "w") as f:
    # This will overwrite the contents of /etc/shadow, a critical system file.
    f.write("corrupted_data")
```

## Safe Code Example

To safely create temporary files, use `tempfile.mkstemp()`, `tempfile.NamedTemporaryFile`, or `tempfile.TemporaryDirectory`. These functions are secure because they create the file or directory atomically, returning a file handle or a path to an already existing object.

### Using `mkstemp()`
This function creates a temporary file and returns a low-level file handle and the absolute pathname. This is the most secure way to create a temporary file.

```python
import tempfile
import os

# mkstemp creates the file and returns a handle and path.
# There is no race condition.
fd, path = tempfile.mkstemp()

try:
    with os.fdopen(fd, 'w') as tmp:
        # Write to the temporary file
        tmp.write('This is safe.\n')
finally:
    # You are responsible for cleaning up the file.
    os.remove(path)
```

### Using `NamedTemporaryFile`
This is a higher-level and often more convenient approach. It creates a file that is automatically deleted when the file object is closed.

```python
import tempfile

# The 'delete=True' argument ensures the file is deleted on close.
with tempfile.NamedTemporaryFile(mode='w', delete=True) as tmp:
    print(f"Created temporary file: {tmp.name}")
    tmp.write('This will be automatically cleaned up.')
    # The file is deleted when the 'with' block exits.
```

## How to Suppress a Finding

There is no good reason to use `tempfile.mktemp()`. It is insecure and has been deprecated for a very long time. You should always migrate to one of the safe alternatives. If for some extreme legacy reason you cannot, you can suppress the finding.

```python
# This is part of a legacy system that cannot be changed.
# The risk of the race condition has been accepted.
# ignore
filename = tempfile.mktemp()
```

Or, for this specific rule:

```python
# ignore: CSP-D504
filename = tempfile.mktemp()
```
