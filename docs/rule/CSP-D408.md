# CSP-D408: Creation of an Unverified SSL/TLS Context

**Vulnerability Category:** `Network`

**Severity:** `MEDIUM`

## Description

This rule flags the use of `ssl._create_unverified_context()`. This function, as its name and underscore prefix imply, is a private, internal function that creates an SSL/TLS context that **does not perform any certificate validation**.

Using this context for a network connection is equivalent to setting `verify=False` and completely disables protection against Man-in-the-Middle (MITM) attacks. An attacker can intercept the connection, presenting a fraudulent certificate, and the client application will accept it without question.

This function was provided as a backward-compatibility measure and should not be used in modern applications.

## Vulnerable Code Example

```python
import urllib.request
import ssl

# This creates a context that disables all certificate checks.
unverified_context = ssl._create_unverified_context()

url = "https://self-signed.badssl.com/"

try:
    # The request is made using the unverified context, opening the door to MITM attacks.
    with urllib.request.urlopen(url, context=unverified_context) as response:
        print("Successfully fetched URL without verification.")
        # An attacker could be intercepting this and reading/modifying the data.
        print(response.read().decode('utf-8'))
except Exception as e:
    print(f"An error occurred: {e}")
```

## Safe Code Example

Always use `ssl.create_default_context()` to get a secure, pre-configured SSL/TLS context that enforces certificate and hostname verification.

```python
import urllib.request
import ssl

# This creates a context with secure defaults.
secure_context = ssl.create_default_context()

# This URL has a valid certificate, so the request will succeed.
url_valid = "https://www.google.com"
# This URL has an invalid certificate, so the request will fail.
url_invalid = "https://self-signed.badssl.com/"


# --- Request to valid URL ---
with urllib.request.urlopen(url_valid, context=secure_context) as response:
    print("Successfully fetched valid URL.")


# --- Request to invalid URL ---
try:
    # This will raise an SSLCertVerificationError because the cert is not trusted.
    with urllib.request.urlopen(url_invalid, context=secure_context) as response:
        pass
except ssl.SSLCertVerificationError as e:
    print(f"Correctly blocked connection to invalid URL: {e}")

```

### Handling Self-Signed Certificates

If you need to connect to a service that uses a self-signed certificate, do not disable verification. Instead, load the specific certificate into the context.

```python
import ssl

# Path to the self-signed certificate file.
cafile = "/path/to/my/self-signed.crt"

# Create a context and load the trusted certificate into it.
context = ssl.create_default_context(cafile=cafile)

# Now, connections made with this context will trust your self-signed cert
# while still rejecting all others.
# with urllib.request.urlopen(url, context=context) as response: ...
```

## How to Suppress a Finding

This function should almost never be used. If you have an exceptional case where you must connect to a system without any certificate validation (e.g., a local test device on an isolated network), you can suppress the finding.

```python
# This is for a local, isolated test device where MITM is not a risk.
# ignore
unverified_context = ssl._create_unverified_context()
```

Or, for this specific rule:

```python
# ignore: CSP-D408
unverified_context = ssl._create_unverified_context()
```
