# CSP-D402: Server-Side Request Forgery (SSRF)

**Vulnerability Category:** `Network`

**Severity:** `CRITICAL`

## Description

Server-Side Request Forgery (SSRF) is a vulnerability that allows an attacker to force a server-side application to make HTTP requests to an arbitrary domain of the attacker's choosing. This can be used to pivot and attack internal, non-public services within the server's network, or to access metadata services in cloud environments (like the AWS or GCP metadata endpoints), potentially leading to a full cloud infrastructure compromise.

This rule flags network requests made with libraries like `requests` or `urllib` where the URL is constructed from user input without proper validation.

## Vulnerable Code Example

```python
import requests
from flask import Flask, request

app = Flask(__name__)

@app.route('/fetch_image')
def fetch_image():
    # The URL is taken directly from a query parameter.
    image_url = request.args.get('url')

    if not image_url:
        return "Please provide a URL.", 400

    try:
        # The application makes a request to the user-provided URL.
        # This is a classic SSRF vulnerability.
        response = requests.get(image_url, timeout=5)
        return response.content, 200, {'Content-Type': response.headers.get('Content-Type')}
    except requests.exceptions.RequestException as e:
        return f"Error fetching image: {e}", 500
```
An attacker could abuse this endpoint by providing URLs like:
- `http://169.254.169.254/latest/meta-data/iam/security-credentials/` to steal AWS credentials.
- `http://localhost:8080/admin` to access an internal admin panel.
- `file:///etc/passwd` to read local files on the server.

## Safe Code Example

The most effective way to prevent SSRF is to use a strict allowlist of trusted domains and protocols. Never allow requests to arbitrary domains.

```python
import requests
from flask import Flask, request
from urllib.parse import urlparse

app = Flask(__name__)

# A strict allowlist of domains the application is allowed to call.
ALLOWED_DOMAINS = {
    'images.example.com',
    'media.trusted-partner.org'
}

@app.route('/fetch_image')
def fetch_image():
    image_url = request.args.get('url')

    if not image_url:
        return "Please provide a URL.", 400

    try:
        parsed_url = urlparse(image_url)

        # 1. Validate the scheme
        if parsed_url.scheme not in ('http', 'https'):
            return "Invalid URL scheme.", 400

        # 2. Validate the domain against the allowlist
        if parsed_url.hostname not in ALLOWED_DOMAINS:
            return "Domain not allowed.", 400

        # 3. Make the request
        response = requests.get(image_url, timeout=5)
        return response.content, 200, {'Content-Type': response.headers.get('Content-Type')}
    except (requests.exceptions.RequestException, ValueError) as e:
        return f"Error fetching image: {e}", 500
```

For more complex scenarios, consider using a well-vetted library specifically designed to prevent SSRF, as blocklists and simple parsing can often be bypassed with clever URL encoding or DNS tricks.

## How to Suppress a Finding

SSRF is a critical vulnerability. Suppressing it is highly discouraged. Only do so if you have implemented robust, custom validation logic that is not automatically detectable.

```python
# The 'validated_url' has been checked against a strict allowlist.
# ignore
response = requests.get(validated_url)
```

Or, for this specific rule:

```python
# ignore: CSP-D402
response = requests.get(validated_url)
```
