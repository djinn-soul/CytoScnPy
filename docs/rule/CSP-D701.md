# CSP-D701: Use of `assert` for Non-Debug Checks

**Vulnerability Category:** `Best Practices`

**Severity:** `LOW`

## Description

This rule flags the use of Python's `assert` statement for checks that are intended to be executed in production. The `assert` statement is primarily intended for debugging and development. When Python is run with the optimization flag (`-O` or `-OO`), all `assert` statements are removed from the compiled bytecode.

If an `assert` statement is used to enforce a security invariant or a critical business logic condition that must always be met, its removal in an optimized environment can lead to unexpected behavior, crashes, or even security vulnerabilities.

## Vulnerable Code Example

```python
def get_user_data(user_id):
    # This check is crucial for security: ensure we don't process invalid IDs.
    # However, if run with python -O script.py, this assertion will be removed.
    assert isinstance(user_id, int) and user_id > 0, "Invalid user ID provided"

    # ... proceed to fetch user data ...
    # If user_id was accidentally passed as None or a negative number,
    # the subsequent code might behave unexpectedly or crash.
    print(f"Fetching data for user ID: {user_id}")
```

## Safe Code Example

For any check that must be enforced regardless of optimization level, use an `if` statement with an explicit `raise` statement. This ensures the check remains active even when Python is run with optimization flags.

```python
def get_user_data(user_id):
    # Use an explicit if statement to raise an exception for invalid input.
    if not (isinstance(user_id, int) and user_id > 0):
        raise ValueError("Invalid user ID provided")

    # ... proceed to fetch user data ...
    print(f"Fetching data for user ID: {user_id}")
```

## When are `assert` statements appropriate?

`assert` statements are perfectly fine for:
- **Debugging:** Checking internal invariants during development.
- **Validating conditions that should never be false:** If an `assert` failing means there's a fundamental bug in your own code that would crash the program anyway, its removal might be acceptable.
- **Input validation that is *only* for development/debugging:** Ensuring that arguments passed to internal functions are correct during testing.

The key is that the check should not be relied upon for production security or critical logic.

## How to Suppress a Finding

If you are using `assert` for its intended purpose (debugging or internal invariant checks) and not for critical production logic, you can suppress this warning.

```python
# This assert is for debugging purposes only.
# ignore
assert x is not None, "x should never be None here during debugging"
```

Or, for this specific rule:

```python
# ignore: CSP-D701
assert some_complex_condition, "This condition must hold during development"
```
