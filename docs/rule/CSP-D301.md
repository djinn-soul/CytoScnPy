# CSP-D301: Use of Weak MD5 Hashing Algorithm

**Vulnerability Category:** `Cryptography`

**Severity:** `MEDIUM`

## Description

This rule flags the use of the MD5 hashing algorithm. MD5 is a widely used cryptographic hash function that produces a 128-bit hash value. However, it has been found to have extensive vulnerabilities and is no longer considered secure for cryptographic purposes.

The primary weaknesses of MD5 are:
1.  **Collision Vulnerability:** It is computationally feasible for an attacker to find two different inputs that produce the same MD5 hash. This can be exploited to create forged digital certificates or to tamper with data without changing its hash.
2.  **Pre-image Resistance:** While harder than finding collisions, it's possible to find an input that generates a given hash, especially with the help of rainbow tables for common inputs like passwords.

MD5 must not be used for security-sensitive applications, including password storage, digital signatures, or data integrity checks.

## Vulnerable Code Example

```python
import hashlib

password = b"my-secret-password"

# Using MD5 to hash a password is insecure.
# The resulting hash is susceptible to collision and pre-image attacks.
hashed_password = hashlib.md5(password).hexdigest()

print(f"MD5 Hash: {hashed_password}")
```

## Safe Code Example

Use a modern, strong hashing algorithm like SHA-256 or SHA-3. For password hashing, it is crucial to also use a salt and a key-derivation function like `scrypt` or `PBKDF2`.

### For General Hashing (Data Integrity)

```python
import hashlib

data = b"some important data"

# Use SHA-256 for a secure hash
hashed_data = hashlib.sha256(data).hexdigest()

print(f"SHA-256 Hash: {hashed_data}")
```

### For Password Hashing

Never store passwords directly. Use a library like `passlib` or Python's built-in `hashlib.scrypt` or `hashlib.pbkdf2_hmac` which are designed for this purpose.

```python
import hashlib
import os

password = b"my-super-secret-password"
salt = os.urandom(16) # A new random salt should be generated for each password

# Use scrypt, a password-based key derivation function
hashed_password = hashlib.scrypt(
    password,
    salt=salt,
    n=16384, # CPU/memory cost factor
    r=8,     # Block size
    p=1      # Parallelization factor
)

# Store the salt along with the hash
```

## When is MD5 acceptable?

MD5 is only acceptable for non-cryptographic purposes, such as a key in a hash table or as a quick, non-security-related checksum for detecting accidental data corruption. If the goal is to protect against malicious tampering, MD5 is not sufficient.

## How to Suppress a Finding

If you are using MD5 for a valid, non-security-related purpose, you can suppress the finding.

```python
import hashlib

# This hash is used as a non-security cache key.
# ignore
cache_key = hashlib.md5(data).hexdigest()
```

Or, for this specific rule:

```python
# ignore: CSP-D301
cache_key = hashlib.md5(data).hexdigest()
```
