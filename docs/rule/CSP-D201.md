# CSP-D201: Insecure Deserialization with Pickle

**Vulnerability Category:** `Deserialization`

**Severity:** `CRITICAL`

## Description

This rule flags the use of `pickle`, `dill`, `shelve`, and `jsonpickle` for deserializing data from untrusted sources. The pickle format is not secure and can be used to execute arbitrary code. An attacker who can control the data being unpickled can craft a malicious payload that will run commands with the privileges of the application.

Never unpickle data from an untrusted or unauthenticated source.

This rule applies to:
- `pickle.load()`, `pickle.loads()`
- `dill.load()`, `dill.loads()`
- `shelve.open()`
- `jsonpickle.decode()`
- `pandas.read_pickle()`

## Vulnerable Code Example

```python
import pickle
import os
import base64

# An attacker crafts a malicious pickle payload
class Exploit:
    def __reduce__(self):
        return (os.system, ('rm -rf /',))

malicious_payload = base64.b64encode(pickle.dumps(Exploit()))

# In a real scenario, this payload might be received from a file,
# a network socket, or a web request.
data_from_attacker = base64.b64decode(malicious_payload)

# This line will execute os.system('rm -rf /')
unpickled_data = pickle.loads(data_from_attacker)
```

## Safe Code Example

The safest approach is to use a secure data format like JSON, MessagePack, or Protobufs for data serialization, especially when dealing with data from external sources. These formats only support simple data types and cannot be used to execute code.

```python
import json

# Original data
my_data = {'name': 'Alice', 'score': 100}

# Serialize to JSON (a safe format)
serialized_data = json.dumps(my_data)

# --- Send over network / save to file ---

# Deserialize from JSON
# This is safe because JSON cannot represent executable code.
received_data = json.loads(serialized_data)

print(received_data)
```

## What if I must use pickle?

If you absolutely must use pickle, you must ensure the data is trustworthy. This can be achieved by cryptographically signing the data before serialization and verifying the signature before deserialization.

```python
import pickle
import hmac
import hashlib

SECRET_KEY = b'my-super-secret-key'

# --- On the trusted side (sender) ---
data_to_serialize = {'user_id': 123, 'role': 'guest'}
pickled_data = pickle.dumps(data_to_serialize)

# Create a signature
signature = hmac.new(SECRET_KEY, pickled_data, hashlib.sha256).hexdigest()

# Send the pickled_data and the signature together

# --- On the untrusted side (receiver) ---
received_pickle = pickled_data # from the sender
received_signature = signature # from the sender

# Verify the signature
expected_signature = hmac.new(SECRET_KEY, received_pickle, hashlib.sha256).hexdigest()
if not hmac.compare_digest(expected_signature, received_signature):
    raise ValueError("Invalid signature. Pickle data may be tampered.")

# It is now safe to unpickle the data
data = pickle.loads(received_pickle)
print(data)
```

## How to Suppress a Finding

If you have verified the data is trusted (e.g., via a signature as shown above), you can suppress the finding.

```python
# The data's signature has been verified.
# ignore
data = pickle.loads(verified_data)
```

Or, for this specific rule:

```python
# ignore: CSP-D201
data = pickle.loads(verified_data)
```
