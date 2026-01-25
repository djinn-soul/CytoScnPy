# CSP-D304: Use of Insecure Cryptographic Ciphers

**Vulnerability Category:** `Cryptography`

**Severity:** `HIGH`

## Description

This rule flags the use of weak or broken symmetric encryption ciphers. Using outdated ciphers can expose sensitive data to decryption by an attacker. Modern computing power and advances in cryptanalysis have rendered many older algorithms insecure.

This rule specifically targets the use of:
- **DES (Data Encryption Standard):** Has a small 56-bit key size, making it vulnerable to brute-force attacks.
- **Triple DES (3DES):** While more secure than DES, it is slow and has known vulnerabilities.
- **RC2, RC4 (ARC4):** Have been found to have several cryptographic weaknesses and biases that can be exploited.
- **Blowfish:** Suffers from a small 64-bit block size, making it vulnerable to attacks on large amounts of data.

For all new applications, **AES (Advanced Encryption Standard)** is the recommended and industry-standard symmetric cipher.

## Vulnerable Code Example (pycryptodome with DES)

```python
from Crypto.Cipher import DES
from Crypto.Random import get_random_bytes

key = get_random_bytes(8) # DES uses a 64-bit (8 byte) key, but only 56 bits are effective
cipher = DES.new(key, DES.MODE_ECB)
data = b'secret data to be hidden'

# Pad data to be a multiple of 8 bytes
padded_data = data + b' ' * (8 - len(data) % 8)

encrypted_data = cipher.encrypt(padded_data)

print(f"Encrypted: {encrypted_data.hex()}")
```

## Safe Code Example (pycryptodome with AES)

Use AES with a secure mode of operation like GCM, which provides both confidentiality and authenticity.

```python
from Crypto.Cipher import AES
from Crypto.Random import get_random_bytes

key = get_random_bytes(16) # AES-128 uses a 16-byte key
cipher = AES.new(key, AES.MODE_GCM)

data = b'secret data to be hidden'

encrypted_data, tag = cipher.encrypt_and_digest(data)

# You must store or transmit the nonce and the tag along with the ciphertext
# nonce = cipher.nonce
# tag = tag

print(f"Encrypted: {encrypted_data.hex()}")
print(f"Tag: {tag.hex()}")
print(f"Nonce: {cipher.nonce.hex()}")

# ... later, to decrypt ...
# a_cipher = AES.new(key, AES.MODE_GCM, nonce=nonce)
# decrypted_data = a_cipher.decrypt_and_verify(encrypted_data, tag)
```

## How to Suppress a Finding

Using a weak cipher is highly discouraged. Suppression should only be considered if you are required to interact with a legacy system that cannot be upgraded.

```python
from Crypto.Cipher import DES

# This is required for compatibility with a legacy hardware device.
# The risk has been assessed and accepted.
# ignore
cipher = DES.new(legacy_key, DES.MODE_ECB)
```

Or, for this specific rule:

```python
# ignore: CSP-D304
cipher = DES.new(legacy_key, DES.MODE_ECB)
```
