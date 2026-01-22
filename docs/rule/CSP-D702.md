# CSP-D702: Import of Insecure or Deprecated Modules

**Vulnerability Category:** `Best Practices`

**Severity:** `HIGH` / `LOW` (depending on module)

## Description

This rule flags the import of modules that are known to be insecure, deprecated, or pose significant security risks. Using these modules can inadvertently introduce vulnerabilities into an application.

Common examples include:

- **`telnetlib`:** Transmits data in cleartext, making it vulnerable to sniffing. Use SSH (`paramiko`) instead ([CSP-D409](./CSP-D409.md)).
- **`ftplib`:** Transmits data and credentials in cleartext. Use SFTP (`paramiko`) or FTPS (`ftplib.FTP_TLS`) instead ([CSP-D406](./CSP-D406.md)).
- **`xml.etree.ElementTree`, `xml.dom.minidom`, `xml.sax`:** These standard libraries can be vulnerable to XML External Entity (XXE) attacks if parsing untrusted input. Use `defusedxml` instead ([CSP-D104](./CSP-D104.md)).
- **`pickle`:** Can lead to arbitrary code execution when deserializing untrusted data. Use JSON or `safe_load` for YAML instead ([CSP-D201](./CSP-D201.md)).
- **`yaml.load` without `Loader`:** Vulnerable to code execution. Use `yaml.safe_load` ([CSP-D202](./CSP-D202.md)).
- **`os.tempnam`, `os.tmpnam`:** Insecure for temporary file creation due to race conditions. Use `tempfile` module instead ([CSP-D506](./CSP-D506.md)).
- **`subprocess` with `shell=True`:** Vulnerable to command injection. Pass arguments as a list instead ([CSP-D003](./CSP-D003.md)).

The severity varies depending on the module; for example, importing `telnetlib` is a higher risk than importing a module that is merely deprecated but not inherently insecure.

## Vulnerable Code Example

```python
# Importing insecure modules for network communication
import telnetlib
import ftplib

# Importing a module that can lead to code execution if used insecurely
import pickle

# Importing modules that are not secure for XML parsing
import xml.etree.ElementTree as ET
```

## Safe Code Example

Replace insecure modules with their secure and modern alternatives.

```python
# Secure alternatives for network communication
import requests # For HTTP
import paramiko # For SSH/SFTP

# Secure data serialization
import json

# Secure XML parsing
import defusedxml.ElementTree as ET

# Secure temporary file handling
import tempfile

# Secure subprocess execution
import subprocess
```

## How to Suppress a Finding

If you are absolutely required to use a legacy or insecure module for compatibility reasons with an external system, and you have performed a thorough risk assessment, you may suppress the finding.

```python
# This import is required to communicate with a legacy device that only supports FTP.
# The network is isolated and credentials are not being sent.
# ignore
import ftplib
```

Or, for this specific rule:

```python
# ignore: CSP-D702
import telnetlib
```
