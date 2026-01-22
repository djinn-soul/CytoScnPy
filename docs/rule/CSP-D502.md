# CSP-D502: Insecure Extraction of `tarfile` Archives (Zip Slip)

**Vulnerability Category:** `Filesystem`

**Severity:** `HIGH`

## Description

This rule flags the insecure extraction of `.tar` archives using the `tarfile` module. A maliciously crafted tar archive can contain files with path traversal sequences in their filenames (e.g., `../../../../tmp/pwned`).

When an application extracts such an archive without proper validation, it can lead to a "Zip Slip" vulnerability. This allows the attacker to write a file anywhere on the filesystem that the application has write access to. This could lead to overwriting critical system files, application source code, or placing a webshell to achieve remote code execution.

This rule specifically targets `tarfile.extract()` and `tarfile.extractall()` when used without safeguards.

## Vulnerable Code Example

```python
import tarfile
import os

# Assume 'malicious.tar' is an untrusted archive.
# It contains a file with the name "../../../home/user/.bashrc"
# and the content "echo 'You have been pwned'"

with tarfile.open("malicious.tar") as tar:
    # This call is vulnerable. It will extract the malicious file
    # outside of the intended 'output_dir', overwriting a user's .bashrc file.
    tar.extractall(path="output_dir")
```

## Safe Code Example (Python 3.12+)

Starting in Python 3.12, the `tarfile` module includes a `filter` argument to control which files can be extracted. The `'data'` filter is designed to prevent the most common security issues, including path traversal.

```python
import tarfile

with tarfile.open("archive.tar") as tar:
    # The 'data' filter disallows path traversal and other dangerous features.
    tar.extractall(path="output_dir", filter='data')
```

## Safe Code Example (Older Python Versions)

For Python versions before 3.12, you must manually inspect each file in the archive before extracting it to ensure it is within the intended destination directory.

```python
import tarfile
import os

destination_dir = "output_dir"

with tarfile.open("archive.tar") as tar:
    for member in tar.getmembers():
        # Resolve the real path of the intended extraction location
        member_path = os.path.join(destination_dir, member.name)
        real_member_path = os.path.realpath(member_path)

        real_destination_dir = os.path.realpath(destination_dir)

        # Check if the resolved path is within the destination directory
        if not real_member_path.startswith(real_destination_dir):
            print(f"Illegal path in tar archive: {member.name}")
            continue

        # The path is safe, so extract it
        tar.extract(member, path=destination_dir)
```
This manual check ensures that no file can be written outside of the `destination_dir`.

## How to Suppress a Finding

You should only suppress this finding if you are extracting an archive from a fully trusted source, or if you have implemented custom validation logic similar to the safe example for older Python versions.

```python
import tarfile

# This archive is generated internally and is known to be safe.
# ignore
with tarfile.open("trusted_archive.tar") as tar:
    tar.extractall()
```

Or, for this specific rule:

```python
# ignore: CSP-D502
with tarfile.open("trusted_archive.tar") as tar:
    tar.extractall()
```
