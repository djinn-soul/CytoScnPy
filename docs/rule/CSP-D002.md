# CSP-D002: Use of `exec()`

**Vulnerability Category:** `Code Execution`

**Severity:** `HIGH`

## Description

The `exec()` function in Python is used for the dynamic execution of Python code. It is similar to `eval()`, but `exec()` can execute arbitrary code blocks, including statements, whereas `eval()` can only evaluate a single expression. Using `exec()` with untrusted input is extremely dangerous and can lead to arbitrary code execution and full system compromise.

## Vulnerable Code Example

```python
import os

user_input = input("Enter a command: ")

# The user can enter a malicious string like:
# "os.system('rm -rf /')"
exec(user_input)

```
In this example, the user can provide a string that, when executed, will run a dangerous command with the privileges of the application.

## Safe Code Example

The safest approach is to avoid `exec()` entirely. If you need to execute different code paths based on user input, use explicit dispatching, such as a dictionary of functions.

```python
def say_hello():
    print("Hello!")

def say_goodbye():
    print("Goodbye!")

commands = {
    "hello": say_hello,
    "goodbye": say_goodbye,
}

user_input = input("Enter a command: ")

command_func = commands.get(user_input)
if command_func:
    command_func()
else:
    print("Unknown command.")
```
This approach is much safer as it only allows the execution of pre-defined functions and does not allow for arbitrary code execution.

## How to Suppress a Finding

If you have performed a thorough security review and are confident that the input to `exec()` is properly sanitized and controlled, you can suppress the finding with a comment:

```python
# ignore
exec(sanitized_input)
```

You can also suppress only this specific rule:

```python
# ignore: CSP-D002
exec(sanitized_input)
```
