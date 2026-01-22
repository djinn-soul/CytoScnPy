# CSP-D401: Insecure TLS/SSL Certificate Verification Disabled

**Vulnerability Category:** `Network`

**Severity:** `HIGH`

## Description

This rule flags the use of `verify=False` in calls made with the `requests` library. The `requests` library is a popular tool for making HTTP requests in Python. By default, it verifies the SSL/TLS certificate of the server it is communicating with to ensure you are connecting to the correct server and that the connection is encrypted.

Setting `verify=False` disables this validation. This makes the connection vulnerable to Man-in-the-Middle (MITM) attacks. An attacker on the same network could intercept the request, impersonate the server, and then read or modify any data sent, including sensitive information like passwords, API keys, or personal data.

## Vulnerable Code Example

```python
import requests

api_key = "my-secret-key"
user_data = {"name": "alice", "email": "alice@example.com"}

try:
    # Disabling verification makes this connection insecure.
    response = requests.post(
        "https://api.example.com/data",
        json=user_data,
        headers={"Authorization": f"Bearer {api_key}"},
        verify=False
    )
    print("Data sent successfully.")
except requests.exceptions.RequestException as e:
    print(f"An error occurred: {e}")
```
In this example, an attacker could intercept the connection, steal the `api_key`, and read the `user_data`.

## Safe Code Example

The safest approach is to always leave `verify` at its default value of `True`.

```python
import requests

api_key = "my-secret-key"
user_data = {"name": "alice", "email": "alice@example.com"}

try:
    # With verify=True (the default), requests will validate the certificate.
    response = requests.post(
        "https://api.example.com/data",
        json=user_data,
        headers={"Authorization": f"Bearer {api_key}"}
    )
    response.raise_for_status() # Raise an exception for bad status codes
    print("Data sent successfully.")
except requests.exceptions.RequestException as e:
    print(f"An error occurred: {e}")
```

### Dealing with Self-Signed Certificates

In development or when connecting to internal systems, you might encounter self-signed certificates. Instead of using `verify=False`, you should provide the path to the certificate authority (CA) bundle or the self-signed certificate file.

```python
import requests

# Provide the path to your custom CA bundle or certificate file.
response = requests.get("https://internal.service.local", verify="/path/to/my/ca.crt")
```

## How to Suppress a Finding

Suppressing this finding is strongly discouraged. It should only be done if you are connecting to a non-sensitive, local resource where MITM attacks are not a concern, and you have no other way to validate the connection.

```python
import requests

# This is a local development server with no sensitive data.
# The risk has been assessed.
# ignore
response = requests.get("https://localhost:8443/status", verify=False)
```

Or, for this specific rule:

```python
# ignore: CSP-D401
response = requests.get("https://localhost:8443/status", verify=False)
```
