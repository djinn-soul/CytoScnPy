# CSP-D405: Network Request Without a Timeout

**Vulnerability Category:** `Network`

**Severity:** `MEDIUM`

## Description

This rule flags network requests made with libraries like `requests` or `urllib` that do not have an explicit timeout configured. By default, these libraries can wait indefinitely for a response from the server.

If the remote server is slow, unresponsive, or malicious, a request without a timeout can cause the client application to hang. In a multi-threaded or multi-process application (like a web server), many such hanging requests can consume all available workers, leading to a Denial of Service (DoS) where the application can no longer handle new requests.

It is a critical best practice to always set a reasonable timeout on all external network requests.

## Vulnerable Code Example

```python
import requests

try:
    # This request has no timeout. If the server at 'https://api.example.com'
    # is down or hangs, this call will block forever.
    response = requests.get("https://api.example.com/data")
    print("Request successful.")
except requests.exceptions.RequestException as e:
    print(f"An error occurred: {e}")
```

## Safe Code Example

Always provide a `timeout` argument to your request calls. The timeout can be a single float value for the entire request, or a tuple of `(connect_timeout, read_timeout)`.

```python
import requests

try:
    # Set a timeout of 5 seconds. If the server doesn't respond within
    # that time, a `requests.exceptions.Timeout` exception will be raised.
    response = requests.get("https://api.example.com/data", timeout=5)
    print("Request successful.")
except requests.exceptions.Timeout:
    print("The request timed out.")
except requests.exceptions.RequestException as e:
    print(f"An error occurred: {e}")
```

### Connect vs. Read Timeout

For more granular control, you can set separate timeouts for connecting to the server and for reading the response.

```python
import requests

try:
    # Wait max 3.5 seconds to establish a connection, and max 10 seconds
    # to receive the response after the connection is made.
    response = requests.get(
        "https://api.example.com/large-file",
        timeout=(3.5, 10)
    )
    print("Request successful.")
except requests.exceptions.Timeout:
    print("The request timed out.")
```

## How to Suppress a Finding

There are very few reasons to make a request without a timeout. If you are interacting with a highly trusted, local service where timeouts are not a concern, you might suppress this. However, it is still better to set a very long timeout than no timeout at all.

```python
# This is a trusted local service that is guaranteed to be responsive.
# ignore
response = requests.get("http://localhost/internal-service")
```

Or, for this specific rule:

```python
# ignore: CSP-D405
response = requests.get("http://localhost/internal-service")
```
