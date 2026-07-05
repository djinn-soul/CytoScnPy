# CSP-D412: MCP StdioServerParameters Non-Literal Command

**Vulnerability Category:** `Code Execution`

**Severity:** `HIGH`

## Description

This rule flags `StdioServerParameters(command=...)` when the command is not a string literal. MCP stdio servers launch a local process. If the command value is dynamic or user-controlled, an attacker may be able to choose an executable and gain arbitrary OS command execution.

## Vulnerable Code Example

```python
from mcp.client.stdio import StdioServerParameters

def connect(server_command):
    return StdioServerParameters(command=server_command, args=["serve"])
```

The executable comes from a variable, so the process that gets launched is not fixed in code.

## Safe Code Example

```python
from mcp.client.stdio import StdioServerParameters

server = StdioServerParameters(
    command="python",
    args=["-m", "trusted_mcp_server"],
)
```

Keep the executable literal and trusted. Put variable data in validated arguments only when the server expects it.

## How to Suppress a Finding

Suppress only when the command is selected from a strict allowlist before constructing `StdioServerParameters`.

```python
# command is selected from a fixed allowlist of internal binaries.
# ignore: CSP-D412
server = StdioServerParameters(command=command, args=args)
```
