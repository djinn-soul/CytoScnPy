# CSP-D407: Use of `HTTPSConnection` Without a Secure SSL Context

**Vulnerability Category:** `Network`

**Severity:** `MEDIUM`

## Description

This rule flags the use of `http.client.HTTPSConnection` without providing a specific `context` parameter.

In older versions of Python (before 2.7.9 and 3.4.3), the default behavior for `HTTPSConnection` was to **not verify** the server's TLS certificate. This is equivalent to `verify=False` in the `requests` library and makes the connection vulnerable to Man-in-the-Middle (MITM) attacks (see [CSP-D401](./CSP-D401.md)).

While modern Python versions have improved defaults, explicitly creating and providing a secure SSL context is the most robust way to ensure your connection is secure across all Python versions and environments. Relying on defaults can be risky if your code needs to run on different systems.

## Vulnerable Code Example

```python
import http.client

# On older Python versions, this connection would not verify the server's
# certificate, making it vulnerable to MITM attacks.
# On newer versions, it uses a default context, but it's better to be explicit.
conn = http.client.HTTPSConnection("api.example.com")

conn.request("GET", "/")
response = conn.getresponse()
print(response.status, response.reason)
data = response.read()
conn.close()
```

## Safe Code Example

Create a secure SSL context using the `ssl` module and pass it to the `HTTPSConnection` constructor. `ssl.create_default_context()` provides a good balance of security and compatibility.

```python
import http.client
import ssl

# Creates a context with secure default settings:
# - Certificate validation enabled
# - Hostname checking enabled
# - Disables insecure protocols like SSLv2 and SSLv3
context = ssl.create_default_context()

try:
    # Providing a context ensures the connection is secure.
    conn = http.client.HTTPSConnection("api.example.com", context=context)

    conn.request("GET", "/")
    response = conn.getresponse()
    print(response.status, response.reason)
    data = response.read()
    conn.close()
except ssl.SSLCertVerificationError as e:
    print(f"Certificate verification failed: {e}")
except ConnectionRefusedError:
    print("Connection refused.")
```

## How to Suppress a Finding

If you are connecting to a trusted local service and have accepted the risk of not performing certificate validation, you can suppress this finding. However, the recommended approach is always to provide a context, even if it's a context configured to trust a specific self-signed certificate.

```python
import http.client

# This connects to a local, trusted service where certificate
# validation is not required.
# ignore
conn = http.client.HTTPSConnection("localhost:8443")
```

Or, for this specific rule:

```python
# ignore: CSP-D407
conn = http.client.HTTPSConnection("localhost:8443")
```
