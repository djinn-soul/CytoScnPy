# CSP-D410: Unvalidated URL Schemes in `urllib.request.urlopen`

**Vulnerability Category:** `Network`

**Severity:** `MEDIUM`

## Description

This rule flags the use of `urllib.request.urlopen` with a URL that may be controlled by a user. The `urlopen` function is a powerful tool that can handle various URL schemes, including `http://`, `https://`, `ftp://`, and importantly, `file://`.

If an attacker can control the URL passed to `urlopen`, they can provide a `file://` URL to read arbitrary files from the local filesystem. This can lead to the exposure of sensitive data, source code, or configuration files.

While related to Server-Side Request Forgery ([CSP-D402](./CSP-D402.md)), this rule specifically focuses on the risk of local file inclusion through schemes other than `http` or `https`.

## Vulnerable Code Example

```python
from urllib.request import urlopen
from flask import Flask, request

app = Flask(__name__)

@app.route('/proxy')
def proxy():
    # The URL comes directly from user input.
    url = request.args.get('url')

    if not url:
        return "Please provide a URL.", 400

    try:
        # An attacker can provide url=file:///etc/passwd
        # This will read the passwd file from the server's filesystem.
        with urlopen(url) as response:
            content = response.read()
            return content
    except Exception as e:
        return f"Error fetching URL: {e}", 500

```

## Safe Code Example

The best way to mitigate this is to parse the URL and validate its components, especially the scheme, before making the request. Only allow schemes that are explicitly required, such as `http` and `https`.

```python
from urllib.request import urlopen
from urllib.parse import urlparse
from flask import Flask, request

app = Flask(__name__)

ALLOWED_SCHEMES = {'http', 'https'}

@app.route('/proxy')
def proxy():
    url = request.args.get('url')

    if not url:
        return "Please provide a URL.", 400

    try:
        parsed_url = urlparse(url)

        # Explicitly check that the scheme is in the allowlist.
        if parsed_url.scheme not in ALLOWED_SCHEMES:
            return "Invalid URL scheme provided.", 400

        with urlopen(url) as response:
            content = response.read()
            return content
    except Exception as e:
        return f"Error fetching URL: {e}", 500
```
By checking the scheme, you prevent `urlopen` from being used to access local files or other unintended resources. For a more robust solution against all forms of SSRF, combine this with a domain allowlist as shown in [CSP-D402](./CSP-D402.md).

## How to Suppress a Finding

If you are certain that the URL being passed to `urlopen` is from a trusted, static source and cannot be manipulated by a user, you can suppress this finding.

```python
from urllib.request import urlopen

# This URL is static and trusted.
# ignore
with urlopen("https://www.python.org/") as response:
    print(response.read())
```

Or, for this specific rule:

```python
# ignore: CSP-D410
with urlopen("https://www.python.org/") as response:
    print(response.read())
```
