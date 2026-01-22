# CSP-D601: Type-Based Method Misuse

**Vulnerability Category:** `Type Safety`

**Severity:** `HIGH`

## Description

This rule identifies a specific and dangerous type of logic error where a method is called on a variable that has a different type than expected. This often happens in dynamically typed languages like Python where a variable's type can change during its lifecycle.

While this might seem like a simple bug, it can have severe security consequences. If a variable that is expected to be a safe, custom object is replaced with a basic type like a string or a list, subsequent method calls can fail in unexpected ways or, in some cases, have unintended side effects.

This is a form of type confusion, where the program's logic proceeds with incorrect assumptions about the data it is processing. CytoScnPy's type inference engine tracks the possible types of a variable and flags when a method call is impossible for a given type, which is a strong indicator of such a flaw.

## Vulnerable Code Example

Consider a function that processes a user profile object.

```python
class UserProfile:
    def __init__(self, username, permissions):
        self.username = username
        self.permissions = permissions

    def has_permission(self, perm):
        return perm in self.permissions

def grant_access(user):
    # This check is intended to work on a UserProfile object.
    if user.has_permission("admin"):
        print("Admin access granted.")
    else:
        print("Standard access granted.")

# --- Later, in a different part of the code ---

# A developer mistakenly passes a username (a string) instead of the user object.
# The type of 'user' has changed from UserProfile to str.
user_object = UserProfile("alice", ["read"])
current_user = user_object.username # Mistake: should be current_user = user_object

# When grant_access is called, it will crash with an AttributeError
# because a string has no 'has_permission' method.
try:
    grant_access(current_user)
except AttributeError as e:
    print(f"Caught an error: {e}")
```
In this case, the error prevents the access check from completing, which might lead to a denial of service. In other, more complex scenarios, the type confusion could lead to logic bypasses. For example, if the check was `if user and user.is_active`, and `user` was an empty list `[]` instead of a `User` object, the check might fail unexpectedly.

## How CytoScnPy Detects This

CytoScnPy analyzes the code and determines that the `current_user` variable holds a string. When it sees the call `grant_access(current_user)`, it knows that inside `grant_access`, the `user` parameter will be a string. It then flags the `user.has_permission("admin")` call as a `CSP-D601` violation because the `str` type has no `has_permission` method.

## Safe Code Example

The solution is to ensure that variables retain their expected types. This can be achieved through careful coding and, more robustly, by using static type annotations and a type checker like Mypy.

```python
from typing import List

class UserProfile:
    def __init__(self, username: str, permissions: List[str]):
        self.username = username
        self.permissions = permissions

    def has_permission(self, perm: str) -> bool:
        return perm in self.permissions

# Using type hints makes the expected type clear.
def grant_access(user: UserProfile):
    if user.has_permission("admin"):
        print("Admin access granted.")
    else:
        print("Standard access granted.")

user_object = UserProfile("alice", ["read"])
# The code now correctly passes the object, not just the username.
current_user: UserProfile = user_object

grant_access(current_user)
```

## How to Suppress a Finding

This finding indicates a definite bug or logic flaw in your code that should be fixed. It is not a matter of style or risk assessment. You should correct the type confusion rather than suppress the warning. If you must suppress it, you can use the standard ignore comments.

```python
# This is known to be incorrect but cannot be fixed right now.
# ignore: CSP-D601
grant_access(current_user) # where current_user is a string
```
