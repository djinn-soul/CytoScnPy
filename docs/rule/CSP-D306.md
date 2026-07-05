# CSP-D306: PyNaCl Low-Level Bindings

**Vulnerability Category:** `Cryptography`

**Severity:** `HIGH`

## Description

This rule flags imports or calls through `nacl.bindings`, the low-level PyNaCl interface to raw NaCl primitives.

Low-level cryptographic primitives are easy to misuse. Incorrect nonce handling, primitive selection, buffer sizing, or authentication checks can silently weaken confidentiality or integrity. Most application code should use PyNaCl's high-level abstractions instead.

## Vulnerable Code Example

```python
from nacl.bindings import crypto_box_open

def decrypt(ciphertext, nonce, public_key, private_key):
    return crypto_box_open(ciphertext, nonce, public_key, private_key)
```

This directly uses the low-level primitive and requires the caller to handle all cryptographic invariants correctly.

## Safe Code Example

```python
from nacl.public import Box, PrivateKey, PublicKey

def decrypt(ciphertext, nonce, public_key_bytes, private_key_bytes):
    box = Box(PrivateKey(private_key_bytes), PublicKey(public_key_bytes))
    return box.decrypt(ciphertext, nonce)
```

Prefer high-level APIs such as `nacl.secret.SecretBox`, `nacl.public.Box`, and `nacl.signing.SigningKey`.

## How to Suppress a Finding

Suppress only in narrowly reviewed cryptographic library code where low-level primitives are intentionally wrapped behind a safer local API.

```python
# Low-level primitive is wrapped in this internal crypto module.
# ignore: CSP-D306
from nacl.bindings import crypto_aead_xchacha20poly1305_ietf_encrypt
```
