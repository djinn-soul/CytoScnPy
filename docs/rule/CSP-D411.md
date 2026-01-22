# CSP-D411: Use of Deprecated `ssl.wrap_socket`

**Vulnerability Category:** `Network`

**Severity:** `MEDIUM`

## Description

This rule flags the use of `ssl.wrap_socket`, which is a deprecated function for adding a TLS/SSL security layer to a standard socket.

Since Python 3.2, this function has been superseded by `SSLContext.wrap_socket()`. The old `ssl.wrap_socket` function has several drawbacks:
- It enables certificate validation by default only in newer Python versions.
- It provides less control over TLS/SSL options, such as protocol versions and cipher suites.
- It is officially deprecated and may be removed in future Python versions.

Using the modern `SSLContext` object provides a much more secure and explicit way to configure TLS/SSL connections.

## Vulnerable Code Example

```python
import socket
import ssl

hostname = 'www.python.org'

# Create a standard TCP socket
sock = socket.create_connection((hostname, 443))

try:
    # Using the deprecated ssl.wrap_socket.
    # On older Python versions, this may not perform certificate validation correctly.
    ssl_sock = ssl.wrap_socket(sock, cert_reqs=ssl.CERT_REQUIRED, ca_certs=None)

    # ... interact with the socket ...

finally:
    ssl_sock.close()
```

## Safe Code Example

The recommended approach is to create a secure `SSLContext` and then use its `wrap_socket()` method. This ensures that secure, modern defaults are used and gives you full control over the configuration.

```python
import socket
import ssl

hostname = 'www.python.org'

# Create a context with secure default settings
context = ssl.create_default_context()

# Create a standard TCP socket
with socket.create_connection((hostname, 443)) as sock:
    # Use the context to wrap the socket
    # This ensures certificate and hostname validation are performed correctly.
    with context.wrap_socket(sock, server_hostname=hostname) as ssl_sock:
        print(f"SSL/TLS version: {ssl_sock.version()}")
        print(f"Cipher: {ssl_sock.cipher()}")

        # ... interact with the socket securely ...
```
This approach is more explicit, secure, and future-proof.

## How to Suppress a Finding

If you are maintaining legacy code that cannot be updated, and you have manually verified that the parameters passed to `ssl.wrap_socket` are secure for your target Python environment, you can suppress this finding. However, migrating to `SSLContext` is strongly recommended.

```python
# This legacy code cannot be changed, and its usage has been verified as safe.
# ignore
ssl_sock = ssl.wrap_socket(sock)
```

Or, for this specific rule:

```python
# ignore: CSP-D411
ssl_sock = ssl.wrap_socket(sock)
```
