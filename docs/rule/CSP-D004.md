# CSP-D004: Async Command Injection

**Vulnerability Category:** `Code Execution`

**Severity:** `CRITICAL`

## Description

This rule is a variant of [CSP-D003](./CSP-D003.md) and applies to asynchronous applications using Python's `asyncio` library. The `asyncio.create_subprocess_shell()` function is vulnerable to command injection in the same way as its synchronous counterparts. An attacker can inject malicious commands by crafting a special input string, leading to arbitrary code execution on the server.

## Vulnerable Code Example

```python
import asyncio

async def run_command(command):
    # The command is passed to the system's shell, creating a vulnerability
    proc = await asyncio.create_subprocess_shell(
        command,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE
    )

    stdout, stderr = await proc.communicate()

    if stdout:
        print(f'[stdout]\n{stdout.decode()}')
    if stderr:
        print(f'[stderr]\n{stderr.decode()}')

async def main():
    user_input = input("Enter a command to run: ")
    # Attacker can input: "echo hello; whoami"
    await run_command(user_input)

if __name__ == "__main__":
    asyncio.run(main())
```
The user-provided `command` string is executed in a shell, allowing an attacker to chain commands.

## Safe Code Example

To mitigate this risk, use `asyncio.create_subprocess_exec()` instead. This function takes the command and its arguments as a list, which prevents the shell from interpreting user input as commands.

```python
import asyncio
import shlex

async def run_command(command_parts):
    # The command and arguments are passed as a list
    proc = await asyncio.create_subprocess_exec(
        *command_parts,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE
    )

    stdout, stderr = await proc.communicate()

    if stdout:
        print(f'[stdout]\n{stdout.decode()}')
    if stderr:
        print(f'[stderr]\n{stderr.decode()}')

async def main():
    user_input = input("Enter a file to display: ")
    # Example: user enters "my_document.txt"
    # It is safely split and quoted
    command_parts = ["cat", shlex.quote(user_input)]
    await run_command(command_parts)

if __name__ == "__main__":
    asyncio.run(main())
```

## How to Suppress a Finding

This is a critical vulnerability and should not be suppressed. If you have rigorously validated the input and must use a shell, you can add a suppression comment.

```python
# ignore
proc = await asyncio.create_subprocess_shell(validated_command)
```

Or, for this specific rule:

```python
# ignore: CSP-D004
proc = await asyncio.create_subprocess_shell(validated_command)
```
