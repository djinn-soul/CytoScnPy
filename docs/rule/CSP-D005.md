# CSP-D005: Use of insecure `input()`

**Vulnerability Category:** `Code Execution`

**Severity:** `HIGH`

## Description

This rule identifies the use of the `input()` function. In legacy Python (2.x) environments, `input()` is equivalent to `eval(raw_input())`, which means any string provided by the user is executed as Python code, creating a severe code execution vulnerability.

While `input()` is safe in Python 3.x (where it returns a string and does not evaluate it), using it can still be a risk if your code is ever run in a legacy or misconfigured environment where `input()` is interpreted in the Python 2 way.

## Vulnerable Code Example

```python
# The input() function evaluates user input in legacy environments.
user_data = input("Please enter your name: ")

# If the user enters: __import__('os').system('ls')
# The command could be executed if running in a legacy context.
print(f"Hello, {user_data}")
```

## Safe Code Example

In modern Python 3, `input()` is safe as it always returns a string. However, for maximum security and cross-version compatibility (if applicable), ensure you are validating or sanitizing all user-provided data.

```python
# In modern Python 3, input() returns a string and is safe from ACE.
user_data = input("Please enter your name: ")

# The input is treated as a literal string.
print(f"Hello, {user_data}")
```

## How to Suppress a Finding

If you are certain the code will only ever run in a Python 3 environment and you are comfortable with the usage, you can suppress the warning.

```python
# ignore
user_data = input("Enter expression: ")
```

Or, for this specific rule:

```python
# ignore: CSP-D005
user_data = input("Enter expression: ")
```
