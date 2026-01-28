# CSP-D311: Use of a Weak Pseudo-Random Number Generator (PRNG)

**Vulnerability Category:** `Cryptography`

**Severity:** `LOW`

## Description

This rule flags the use of Python's `random` module for generating random numbers in a security-sensitive context. The `random` module implements a pseudo-random number generator (specifically, the Mersenne Twister), which is perfectly suitable for modeling, simulation, and other non-cryptographic tasks.

However, it is **not** cryptographically secure. Its output is predictable; if an attacker can learn the internal state of the generator, they can predict all future (and past) random numbers. This makes it unsuitable for any security purpose, such as:
- Generating passwords, session tokens, or other credentials.
- Generating cryptographic keys or nonces.
- Creating salts for password hashing.
- Any part of an authentication or authorization mechanism.

## Vulnerable Code Example

```python
import random
import string

def generate_reset_token(length=16):
    # Using random.choice is not secure for generating tokens.
    # An attacker could potentially predict the token.
    chars = string.ascii_letters + string.digits
    return ''.join(random.choice(chars) for _ in range(length))

print(f"Insecure reset token: {generate_reset_token()}")
```

## Safe Code Example

For any security-related need for randomness, use the `secrets` module, which was introduced in Python 3.6. The `secrets` module uses the operating system's most secure source of randomness (`os.urandom()` on most systems) and is designed specifically for cryptographic use.

```python
import secrets
import string

def generate_secure_reset_token(length=16):
    # secrets.choice is cryptographically secure.
    chars = string.ascii_letters + string.digits
    return ''.join(secrets.choice(chars) for _ in range(length))

print(f"Secure reset token: {generate_secure_reset_token()}")

# For generating a URL-safe text string (e.g., for tokens)
secure_token = secrets.token_urlsafe(16) # Creates a 16-byte random token
print(f"URL-safe token: {secure_token}")
```

For generating raw random bytes (e.g., for cryptographic keys), you can use `os.urandom()` or `secrets.token_bytes()`.

```python
import os
import secrets

# Both of these are secure ways to get random bytes.
key1 = os.urandom(16)
key2 = secrets.token_bytes(16)
```

## How to Suppress a Finding

You should only suppress this finding if you are confident the use of the `random` module is for a non-security purpose.

```python
import random

# This is for a simulation, not for security.
# ignore
random_value = random.randint(1, 100)
```

Or, for this specific rule:

```python
# ignore: CSP-D311
random_value = random.random()
```
