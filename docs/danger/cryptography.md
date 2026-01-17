# Category 4: Cryptography & Randomness (CSP-D3xx)

Rules in this category detect weak cryptographic algorithms and insecure random number generation.

| Rule ID      | Pattern                                | Severity   | Why it's risky              | Safer alternative / Fix       |
| :----------- | :------------------------------------- | :--------- | :-------------------------- | :---------------------------- |
| **CSP-D301** | `hashlib.md5`, `hashlib.new('md5')`    | **MEDIUM** | Collision-prone weak hash   | Use SHA-256 or SHA-3          |
| **CSP-D302** | `hashlib.sha1`, `hashlib.new('sha1')`  | **MEDIUM** | Weak cryptographic hash     | Use SHA-256 or SHA-3          |
| **CSP-D304** | Insecure Ciphers (ARC4, DES, Blowfish) | **HIGH**   | Successfully broken ciphers | Use AES (GCM/CTR)             |
| **CSP-D305** | Insecure Cipher Mode (ECB)             | **MEDIUM** | Pattern leakage             | Use CBC or GCM                |
| **CSP-D311** | `random.*` (Standard PRNG)             | **LOW**    | Predictable for crypto use  | Use `secrets` or `os.urandom` |

## In-depth: Weak Hashing (CSP-D301)

MD5 and SHA-1 are considered cryptographically broken and should not be used for security-sensitive purposes like password hashing or digital signatures.

### Dangerous Pattern

```python
import hashlib
h = hashlib.md5(b"password").hexdigest() # INSECURE
```

### Safe Alternative

```python
import hashlib
h = hashlib.sha256(b"password").hexdigest() # SECURE
```
