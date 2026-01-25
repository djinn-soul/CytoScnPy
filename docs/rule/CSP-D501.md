# CSP-D501: Path Traversal

**Vulnerability Category:** `Filesystem`

**Severity:** `HIGH`

## Description

Path traversal (also known as directory traversal) is a web security vulnerability that allows an attacker to read arbitrary files on the server that is running an application. This vulnerability arises when user-supplied input is used to construct a path to a file or directory without proper validation.

An attacker can use path traversal sequences like `../` to navigate outside of the intended base directory. This can lead to the disclosure of sensitive information, such as application source code, configuration files, credentials, or system files.

This rule flags common filesystem operations where user input is used to construct a path, such as `open()`, `os.path.join()`, and methods on `pathlib.Path` objects.

## Vulnerable Code Example

```python
import os
from flask import Flask, request

app = Flask(__name__)

BASE_DIR = '/var/www/uploads'

@app.route('/files')
def get_file():
    # The filename is taken directly from user input.
    filename = request.args.get('filename')

    if not filename:
        return "Please provide a filename.", 400

    # The user input is joined with the base directory.
    # An attacker can provide filename=../../../../etc/passwd
    file_path = os.path.join(BASE_DIR, filename)

    try:
        with open(file_path, 'r') as f:
            return f.read()
    except FileNotFoundError:
        return "File not found.", 404
    except Exception as e:
        return str(e), 500
```
In this example, an attacker can access any file on the system that the web server has read permissions for.

## Safe Code Example

To prevent path traversal, you must validate that the final, resolved path is within the intended base directory. The `os.path.realpath()` or `pathlib.Path.resolve()` methods should be used to resolve any symbolic links or `../` sequences.

```python
import os
from flask import Flask, request

app = Flask(__name__)

BASE_DIR = '/var/www/uploads'

@app.route('/files')
def get_file():
    filename = request.args.get('filename')

    if not filename:
        return "Please provide a filename.", 400

    # Construct the path first
    file_path = os.path.join(BASE_DIR, filename)

    # Resolve the absolute path and check if it's within the base directory
    real_base_dir = os.path.realpath(BASE_DIR)
    real_file_path = os.path.realpath(file_path)

    if not real_file_path.startswith(real_base_dir):
        return "Path traversal attempt detected.", 400

    try:
        with open(real_file_path, 'r') as f:
            return f.read()
    except FileNotFoundError:
        return "File not found.", 404
    except Exception as e:
        return str(e), 500
```

### Using `pathlib` (Python 3.4+)
The `pathlib` module provides a more object-oriented and often clearer way to handle paths.

```python
from pathlib import Path
# ...
base_dir = Path('/var/www/uploads').resolve()
user_path = base_dir / filename
resolved_path = user_path.resolve()

if base_dir not in resolved_path.parents and resolved_path != base_dir:
    return "Path traversal attempt detected.", 400

# with open(resolved_path, 'r') as f: ...
```

## How to Suppress a Finding

If you have implemented custom validation logic that ensures the path is safe before it's used, you can suppress this finding.

```python
# The 'safe_filename' has been validated to contain no path traversal characters.
# ignore
file_path = os.path.join(BASE_DIR, safe_filename)
```

Or, for this specific rule:

```python
# ignore: CSP-D501
with open(validated_path, 'r') as f:
    ...
```
