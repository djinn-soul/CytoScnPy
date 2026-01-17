# Category 3: Deserialization (CSP-D2xx)

Rules in this category detect unsafe deserialization of untrusted data, which can lead to Remote Code Execution (RCE).

| Rule ID      | Pattern                                                        | Severity     | Why it's risky            | Safer alternative / Fix           |
| :----------- | :------------------------------------------------------------- | :----------- | :------------------------ | :-------------------------------- |
| **CSP-D201** | `pickle`, `dill`, `shelve`, `jsonpickle`, `pandas.read_pickle` | **CRITICAL** | Arbitrary code execution  | Use JSON, msgpack, or signed data |
| **CSP-D202** | `yaml.load` (no SafeLoader)                                    | HIGH         | Arbitrary code execution  | `yaml.safe_load(...)`             |
| **CSP-D203** | `marshal.load`/`loads`                                         | **MEDIUM**   | Unsafe for untrusted data | Use secure serialization          |

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
