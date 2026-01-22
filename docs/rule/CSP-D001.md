# CSP-D001: Use of `eval()`

**Vulnerability Category:** `Code Execution`

**Severity:** `HIGH`

## Description

The `eval()` function in Python is a powerful tool that allows for the dynamic execution of Python code from a string. However, it is also a significant security risk when used with untrusted input. An attacker can inject malicious code into the `eval()` function, which will then be executed with the same permissions as the running application. This can lead to a full system compromise.

## Vulnerable Code Example

```python
import os

user_input = input("Enter a value: ")

# The user can enter a malicious string like:
# __import__('os').system('rm -rf /')
result = eval(user_input)

print(f"Result: {result}")
```
In this example, the user can provide a string that, when evaluated, will execute a dangerous command.

## Safe Code Example

In many cases, `eval()` is used to parse simple data structures. The `ast.literal_eval()` function can be used as a safer alternative in these situations. `literal_eval()` will only evaluate a limited set of Python literals, such as strings, numbers, tuples, lists, dicts, booleans, and `None`.

```python
import ast

user_input = input("Enter a value: ")

try:
    # Safely evaluate a literal string
    result = ast.literal_eval(user_input)
    print(f"Result: {result}")
except (ValueError, SyntaxError):
    print("Invalid input.")

```
If you need to execute more complex code, it is better to redesign your application to avoid dynamic code execution. If that is not possible, you must ensure that the input to `eval()` is strictly controlled and sanitized.

## How to Suppress a Finding

If you have assessed the risk and have determined that the use of `eval()` is safe in your specific context, you can suppress the finding by adding a comment to the line of code:

```python
# ignore
result = eval(user_input)
```

You can also be more specific and suppress only this particular rule:

```python
# ignore: CSP-D001
result = eval(user_input)
```
