# CSP-D505: Insecure File Permissions

**Vulnerability Category:** `Filesystem`

**Severity:** `HIGH`

## Description

This rule flags the use of overly permissive file permissions, specifically when a file is made world-writable. Using functions like `os.chmod()` to set file permissions that include `stat.S_IWOTH` allows any user on the system to modify the file's content.

If a world-writable file is used for configuration, application logic (like a Python script), or to store sensitive data, it can lead to a variety of attacks:
- **Arbitrary Code Execution:** An attacker can modify a script or configuration file to execute malicious commands.
- **Data Tampering:** Sensitive application data can be altered or corrupted.
- **Denial of Service:** An attacker could delete the file's content or replace it with garbage, causing the application to crash.

Files should always be created with the most restrictive permissions necessary for the application to function correctly.

## Vulnerable Code Example

```python
import os
import stat

filename = "config.ini"

with open(filename, "w") as f:
    f.write("[database]\nuser = app_user\n")

# This sets the permissions to 666 (rw-rw-rw-), which is insecure.
# Any user on the system can now modify this config file.
os.chmod(filename, stat.S_IRUSR | stat.S_IWUSR | stat.S_IRGRP | stat.S_IWGRP | stat.S_IROTH | stat.S_IWOTH)

# A more common, but still dangerous, way to see this is with octal literals:
# os.chmod(filename, 0o777) # rwxrwxrwx
```

## Safe Code Example

Follow the principle of least privilege. Set permissions that only allow the file's owner to read and write it. In many cases, the default permissions set by `open()` are sufficient and `os.chmod()` is not needed. If you must set permissions, use a restrictive mode.

```python
import os
import stat

filename = "config.ini"

# Create the file (default permissions are usually fine)
with open(filename, "w") as f:
    f.write("[database]\nuser = app_user\n")

# If you need to explicitly set permissions, make them restrictive.
# 600 (rw-------) is a good default for sensitive files.
os.chmod(filename, stat.S_IRUSR | stat.S_IWUSR)

# Using an octal literal is also common and clear:
# os.chmod(filename, 0o600)
```
This ensures that only the user running the application can read or write to the file.

## How to Suppress a Finding

You should only suppress this finding if you are intentionally creating a file that needs to be writable by all users, such as a file in a shared `/tmp` directory for inter-process communication, and you have assessed that the risks of tampering are low or mitigated in another way.

```python
# This file is a temporary IPC mechanism and is designed to be world-writable.
# The data written to it is not sensitive and is validated on read.
# ignore
os.chmod(ipc_file, 0o666)
```

Or, for this specific rule:

```python
# ignore: CSP-D505
os.chmod(ipc_file, 0o666)
```
