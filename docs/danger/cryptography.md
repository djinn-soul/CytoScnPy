# Category 4: Cryptography & Randomness (CSP-D3xx)

Rules in this category detect weak cryptographic algorithms and insecure random number generation.

| Rule ID      | Pattern                            | Severity   | Why it's risky             | Safer alternative / Fix       |
| :----------- | :--------------------------------- | :--------- | :------------------------- | :---------------------------- |
| **CSP-D301** | Weak hashing (MD5, etc.)           | **MEDIUM** | Collision-prone weak hash  | Use SHA-256 or SHA-3          |
| **CSP-D302** | Weak hashing (SHA-1)               | **MEDIUM** | Collision-prone weak hash  | Use SHA-256 or SHA-3          |
| **CSP-D304** | Insecure ciphers (DES, ARC4, etc.) | **HIGH**   | Process/Data compromise    | Use AES                       |
| **CSP-D305** | Insecure cipher modes (ECB)        | **MEDIUM** | Pattern leakage in cipher  | Use CBC or GCM                |
| **CSP-D311** | `random.*` (Standard PRNG)         | **LOW**    | Predictable for crypto use | Use `secrets` or `os.urandom` |

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
