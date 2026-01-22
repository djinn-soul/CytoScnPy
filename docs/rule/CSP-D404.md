# CSP-D404: Service Hardcoded to Bind to All Network Interfaces

**Vulnerability Category:** `Network`

**Severity:** `MEDIUM`

## Description

This rule flags network services that are hardcoded to bind to `0.0.0.0` (for IPv4) or `::` (for IPv6). Binding a server to this address makes it listen on all available network interfaces.

While this is often necessary for services that are intended to be public (like a production web server), it can be dangerous for development servers, test harnesses, or internal tools. If a developer runs a service bound to `0.0.0.0` on their local machine, that service may become exposed to the local network. If the service is not properly secured, this could lead to unauthorized access from others on the same network.

For services intended only for local access, they should be explicitly bound to `127.0.0.1` (localhost).

## Vulnerable Code Example (Flask)

```python
from flask import Flask

app = Flask(__name__)

@app.route('/')
def index():
    return "This is a private development server."

if __name__ == '__main__':
    # Binding to '0.0.0.0' makes the server accessible from the local network.
    # If the developer is on a public Wi-Fi, this could be very dangerous.
    app.run(host='0.0.0.0', port=8080)
```

## Vulnerable Code Example (Socket Server)

```python
import socket

# Create a socket server
server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

# Binding to all interfaces
server_socket.bind(('0.0.0.0', 9999))
server_socket.listen(5)

print("Server listening on all interfaces on port 9999...")
# ... server logic ...
```

## Safe Code Example

For local development and testing, always bind to `127.0.0.1` or `localhost`. This ensures the service is only accessible from the same machine.

### Safe Flask Example
```python
from flask import Flask

app = Flask(__name__)

@app.route('/')
def index():
    return "This is a private development server."

if __name__ == '__main__':
    # Binding to '127.0.0.1' ensures the server is only accessible locally.
    app.run(host='127.0.0.1', port=8080)
```

### Safe Production Practice

In production, it is common to bind to `0.0.0.0` within a container. The exposure is then controlled by the container orchestration system (like Docker or Kubernetes) and the host's firewall rules. In this context, it is acceptable, but the configuration should ideally be managed via environment variables, not hardcoded.

```python
import os

# Get the host from an environment variable, defaulting to localhost.
host = os.environ.get('APP_HOST', '127.0.0.1')
port = int(os.environ.get('APP_PORT', 8080))

app.run(host=host, port=port)
```

## How to Suppress a Finding

If you are intentionally creating a public-facing service and have appropriate firewall rules and security controls in place, you can suppress this finding.

```python
# This is a public service and is intended to be exposed.
# Security is handled by the cloud environment's firewall.
# ignore
app.run(host='0.0.0.0', port=80)
```

Or, for this specific rule:

```python
# ignore: CSP-D404
app.run(host='0.0.0.0', port=80)
```
