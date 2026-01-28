# CSP-D302: Use of Weak SHA-1 Hashing Algorithm

**Vulnerability Category:** `Cryptography`

**Severity:** `MEDIUM`

## Description

This rule flags the use of the SHA-1 (Secure Hash Algorithm 1) hashing algorithm. While once a standard for data integrity and digital signatures, SHA-1 is now considered insecure for cryptographic purposes.

In 2017, researchers demonstrated the first practical collision attack against SHA-1, meaning they were able to create two different files with the same SHA-1 hash. This makes SHA-1 unsuitable for any security-sensitive application where collision resistance is important.

SHA-1 should not be used for:
- Digital signatures
- Password storage
- Data integrity checks in security contexts

## Vulnerable Code Example

```python
import hashlib

data = b"This data should be secure."

# Using SHA-1 is no longer considered safe for cryptographic purposes.
# An attacker could potentially create a different piece of data with the same hash.
hashed_data = hashlib.sha1(data).hexdigest()

print(f"SHA-1 Hash: {hashed_data}")
```

## Safe Code Example

Use a modern, strong hashing algorithm from the SHA-2 or SHA-3 family, such as SHA-256 or SHA-512.

### For General Hashing (Data Integrity)

```python
import hashlib

data = b"This data should be secure."

# Use SHA-256 for a secure hash that is resistant to collisions.
hashed_data = hashlib.sha256(data).hexdigest()

print(f"SHA-256 Hash: {hashed_data}")
```

### For Password Hashing

As with MD5, SHA-1 is not suitable for password storage. Use a dedicated password-based key derivation function like `scrypt` or `PBKDF2`, which incorporates salts and is computationally intensive. See [CSP-D301](./CSP-D301.md) for a password hashing example.

## Is SHA-1 ever acceptable?

The use of SHA-1 is strongly discouraged. Its only potentially acceptable use case is in legacy protocols where it is required for backward compatibility. Even in these cases, it is critical to understand the risks and to plan for an upgrade. For new applications, SHA-1 should not be used.

HMAC-SHA1 is generally considered safe as it is not directly vulnerable to collision attacks, but migrating to HMAC-SHA256 is still the recommended best practice.

## How to Suppress a Finding

If you are required to use SHA-1 for compatibility with a legacy system and have assessed the risks, you can suppress the finding.

```python
import hashlib

# This is required for a legacy API compatibility.
# ignore
legacy_hash = hashlib.sha1(data).hexdigest()
```

Or, for this specific rule:

```python
# ignore: CSP-D302
legacy_hash = hashlib.sha1(data).hexdigest()
```
