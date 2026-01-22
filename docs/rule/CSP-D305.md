# CSP-D305: Use of Insecure Cipher Mode (ECB)

**Vulnerability Category:** `Cryptography`

**Severity:** `MEDIUM`

## Description

This rule flags the use of the Electronic Codebook (ECB) mode for symmetric encryption. Even when used with a strong cipher like AES, ECB mode is insecure because it encrypts identical blocks of plaintext into identical blocks of ciphertext. This means that patterns in the plaintext are preserved in the ciphertext, which can leak significant information about the original data.

An attacker can analyze the encrypted data to identify patterns, which may help them to infer the content of the message. This is famously illustrated by the "ECB penguin" image, where the encrypted image of a penguin is still clearly recognizable.

## Vulnerable Code Example

```python
from Crypto.Cipher import AES

key = b'Sixteen byte key' # 16 bytes = 128 bits
cipher = AES.new(key, AES.MODE_ECB) # Using ECB mode is insecure

# A message with a repeating pattern
data = b'ATTACK ATTACK ATTACK ATTACK'

encrypted_data = cipher.encrypt(data)

# The encrypted data will also have a repeating pattern, leaking information.
print(f"Encrypted: {encrypted_data.hex()}")
```
In the example above, the ciphertext for each "ATTACK" block will be identical.

## Safe Code Example

Use a secure mode of operation that provides confidentiality and, ideally, authenticity. Recommended modes include:
- **GCM (Galois/Counter Mode):** Provides authenticated encryption (AEAD), ensuring both confidentiality and integrity. This is often the best choice.
- **CBC (Cipher Block Chaining):** A widely used mode that chains blocks together, but requires careful handling of Initialization Vectors (IVs) and padding. It does not provide integrity protection on its own.

### Safe Example with GCM

```python
from Crypto.Cipher import AES
from Crypto.Random import get_random_bytes

key = get_random_bytes(16)
data = b'ATTACK ATTACK ATTACK ATTACK'

# GCM is a modern, secure, and recommended mode.
cipher = AES.new(key, AES.MODE_GCM)
encrypted_data, tag = cipher.encrypt_and_digest(data)
nonce = cipher.nonce

# The resulting ciphertext will not have a discernible pattern.
print(f"Encrypted: {encrypted_data.hex()}")

# To decrypt, the nonce and tag are required, which protects against tampering.
# a_cipher = AES.new(key, AES.MODE_GCM, nonce=nonce)
# decrypted_data = a_cipher.decrypt_and_verify(encrypted_data, tag)
```

## How to Suppress a Finding

Using ECB mode is strongly discouraged. It should only be used if required for compatibility with a legacy system that cannot be changed, and only if the data being encrypted has no discernible patterns.

```python
from Crypto.Cipher import AES

# Required for a legacy system. The data is known to be random and has no patterns.
# The risks have been fully assessed.
# ignore
cipher = AES.new(legacy_key, AES.MODE_ECB)
```

Or, for this specific rule:

```python
# ignore: CSP-D305
cipher = AES.new(legacy_key, AES.MODE_ECB)
```
