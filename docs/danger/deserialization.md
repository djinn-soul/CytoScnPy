# Category 3: Deserialization (CSP-D2xx)

Rules in this category detect unsafe deserialization of untrusted data, which can lead to Remote Code Execution (RCE).

| Rule ID      | Pattern                                                        | Severity     | Why it's risky           | Safer alternative / Fix           |
| :----------- | :------------------------------------------------------------- | :----------- | :----------------------- | :-------------------------------- |
| **CSP-D201** | `pickle`, `dill`, `shelve`, `jsonpickle`, `pandas.read_pickle` | **CRITICAL** | Arbitrary code execution | Use JSON, msgpack, or signed data |
| **CSP-D202** | `yaml.load` (no SafeLoader)                                    | **HIGH**     | Arbitrary code execution | `yaml.safe_load(...)`             |
| **CSP-D203** | `marshal.load`/`loads`                                         | **MEDIUM**   | Arbitrary code execution | Use JSON or signed data           |
| **CSP-D204** | `torch.load`, `keras.load_model`, `joblib.load`                | **CRITICAL** | ACE via embedded pickle  | Use `weights_only=True` (torch)   |

## In-depth: ML Model Deserialization (CSP-D204)

Many ML libraries use `pickle` under the hood to load models. Loading a model from an untrusted source can execute arbitrary code on your machine.

### Dangerous Pattern

```python
import torch
model = torch.load("untrusted_model.pt") # VULNERABLE
```

### Safe Alternative

```python
import torch
model = torch.load("untrusted_model.pt", weights_only=True) # SAFE: Only loads tensors
```

## In-depth:marshal Deserialization (CSP-D203)

The `marshal` module is intended for internal Python use and is not secure against malicious data. It can be used to execute arbitrary code.

### Dangerous Pattern

```python
import marshal
data = get_data_from_network()
obj = marshal.loads(data) # DANGEROUS
```

## In-depth: Pickle Deserialization (CSP-D201)

The `pickle` module is NOT secure against erroneous or maliciously constructed data. Never unpickle data received from an untrusted or unauthenticated source.

### Dangerous Pattern

```python
import pickle
data = get_data_from_network()
obj = pickle.loads(data) # EXTREMELY DANGEROUS
```

### Safe Alternative

```python
import json
data = get_data_from_network()
obj = json.loads(data) # SAFE
```
