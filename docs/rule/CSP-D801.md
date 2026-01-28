# CSP-D801: Open Redirect

**Vulnerability Category:** `Open Redirect`

**Severity:** `HIGH`

## Description

An Open Redirect vulnerability occurs when a web application redirects users to a URL that is supplied by an untrusted input, without proper validation. The attacker can craft a URL that includes a malicious redirect target, such as a phishing site or a site that exploits browser vulnerabilities.

When a user clicks on a malicious link, they may be redirected to a site that appears to be legitimate (e.g., `https://your-trusted-app.com/redirect?url=https://malicious-site.com`) but is actually controlled by an attacker. This can be used to trick users into divulging sensitive information or performing unwanted actions.

## Vulnerable Code Example (Flask)

```python
from flask import Flask, redirect, request, url_for

app = Flask(__name__)

@app.route('/redirect_user')
def redirect_user():
    # The 'next' parameter is taken directly from user input.
    next_url = request.args.get('next')

    if next_url:
        # VULNERABLE: The application redirects to an unvalidated URL.
        # An attacker could provide 'next=https://phishing-site.com'
        return redirect(next_url)
    else:
        # Redirect to a safe default if 'next' is not provided.
        return redirect(url_for('index'))

@app.route('/')
def index():
    return "Welcome!"

if __name__ == '__main__':
    app.run(debug=True)
```

## Safe Code Example

To prevent open redirects, always validate user-supplied URLs before performing a redirect. This typically involves:
1.  **Whitelisting allowed domains:** Only permit redirects to domains that are explicitly on a trusted list.
2.  **Ensuring relative paths:** If redirects are only meant for internal application URLs, ensure the provided URL is a relative path and not an absolute URL.
3.  **Providing a safe default:** Always redirect to a known safe page if the provided URL is invalid or missing.

```python
from flask import Flask, redirect, request, url_for
from urllib.parse import urlparse, urljoin

app = Flask(__name__)

# Whitelist of domains that are allowed for redirection.
# This should include your own application's domain(s).
ALLOWED_REDIRECT_HOSTS = ['127.0.0.1:5000', 'your-trusted-app.com']

def is_safe_redirect_url(target):
    """
    Checks if the target URL is safe for redirection.
    It must be relative or point to a host in our ALLOWED_REDIRECT_HOSTS.
    """
    if not target:
        return False

    # Check if it's an absolute URL and if its host is in the allowed list.
    parsed_target = urlparse(target)

    # If scheme or netloc are present, it's an absolute URL.
    # If netloc is empty, it's a relative path.
    if parsed_target.scheme and parsed_target.netloc:
        return parsed_target.netloc in ALLOWED_REDIRECT_HOSTS
    elif not parsed_target.scheme and not parsed_target.netloc:
        # It's a relative path, which is generally safe.
        return True
    else:
        # Other cases like 'http://' or 'https://' without a host are invalid.
        return False

@app.route('/redirect_user')
def redirect_user():
    next_url = request.args.get('next')

    # Validate the URL before redirecting.
    if next_url and is_safe_redirect_url(next_url):
        return redirect(next_url)
    else:
        # Redirect to a safe default page if validation fails.
        return redirect(url_for('index'))

@app.route('/')
def index():
    return "Welcome!"

if __name__ == '__main__':
    app.run(debug=True)
```

## How to Suppress a Finding

This is a critical security vulnerability and should generally not be suppressed. If you have implemented very strict, custom validation logic that is guaranteed to prevent all open redirect attacks, you might suppress it.

```python
# The redirect URL is validated using a custom, secure logic.
# ignore
return redirect(validated_url)
```

Or, for this specific rule:

```python
# ignore: CSP-D801
return redirect(validated_url)
```
