# CSP-D203: Deserialization with `marshal`

**Vulnerability Category:** `Deserialization`

**Severity:** `MEDIUM`

## Description

This rule flags the use of `marshal.load()` and `marshal.loads()`. The `marshal` module is primarily used internally by Python to read and write the compiled `.pyc` files. The Python documentation explicitly warns that the `marshal` format is not secure and should not be used with untrusted data.

While not as straightforward to exploit as `pickle`, a malicious `marshal` payload can crash the Python interpreter, cause a denial of service, or potentially lead to code execution in some versions of Python.

From the [official Python documentation](https://docs.python.org/3/library/marshal.html):
> **Warning:** The `marshal` module is not intended to be secure against erroneous or maliciously constructed data. Never unmarshal data received from an untrusted or unauthenticated source.

## Vulnerable Code Example

```python
import marshal

# Assume 'data_from_network' is received from an untrusted source.
# A malformed or malicious payload can crash the program.
try:
    obj = marshal.loads(data_from_network)
except ValueError:
    print("Could not unmarshal the data.")

```

## Safe Code Example

For serializing Python objects, especially for data interchange, use a secure, standardized format like JSON.

```python
import json

my_data = {"key": "value", "number": 42}

# Serialize with JSON
json_string = json.dumps(my_data)

# --- Send over network / save to file ---

# Deserialize with JSON
# This is safe and robust against malformed data.
try:
    received_data = json.loads(json_string)
    print(received_data)
except json.JSONDecodeError:
    print("Invalid JSON received.")
```

## How to Suppress a Finding

The `marshal` module should almost never be used for data persistence or interchange. Its use is typically restricted to Python's internal workings. If you have a specific, internal use case where you control the data being unmarshalled, you can suppress the finding.

```python
import marshal

# This data is from a trusted, internal source.
# ignore
internal_obj = marshal.loads(trusted_data)
```

Or, for this specific rule:

```python
# ignore: CSP-D203
internal_obj = marshal.loads(trusted_data)
```
