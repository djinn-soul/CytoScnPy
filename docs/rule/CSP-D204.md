# CSP-D204: Insecure Deserialization of Machine Learning Models

**Vulnerability Category:** `Deserialization`

**Severity:** `CRITICAL`

## Description

This rule identifies the insecure loading of machine learning models from libraries like PyTorch, Keras, and Joblib. Many model formats, including `.pth`, `.h5`, and `.pkl`, use Python's `pickle` module under the hood. As described in [CSP-D201](./CSP-D201.md), unpickling data from an untrusted source can lead to arbitrary code execution.

Loading a malicious model file can compromise the system it's running on. Always download models from trusted sources and, when possible, use safer loading methods.

This rule flags:
- `torch.load()` without `weights_only=True`
- `joblib.load()`
- `keras.models.load_model()` on non-Keras native formats.

## Vulnerable Code Example (PyTorch)

```python
import torch

# Assume 'untrusted_model.pth' is downloaded from the internet.
# It could contain a malicious payload in its pickled data.
model = torch.load("untrusted_model.pth")
```

## Vulnerable Code Example (Joblib)

`joblib` is often used to save Scikit-learn models, and it uses pickle by default.

```python
import joblib

# Loading a joblib file is equivalent to unpickling.
model = joblib.load("untrusted_model.pkl")
```

## Safe Code Example (PyTorch)

PyTorch introduced a `weights_only` parameter to safely load only the model's tensor data, ignoring any potentially malicious code.

```python
import torch

# This will safely load the model's weights and biases.
# It will raise an error if the file contains any other pickled objects.
try:
    model_state_dict = torch.load("untrusted_model.pth", weights_only=True)
    # model.load_state_dict(model_state_dict) # then load into your model class
except RuntimeError as e:
    print(f"Blocked potentially malicious model file: {e}")
```

## Safe Practices for All Libraries

For all machine learning model formats, the most important defense is **source verification**.
1.  **Download from Trusted Sources:** Only use models from official repositories or sources you trust (e.g., official PyTorch Hub, Hugging Face with scans enabled, official TensorFlow Hub).
2.  **Verify Hashes:** When downloading a model, verify its hash (SHA256, etc.) against the one provided by the source.
3.  **Use Safe Formats:** Prefer formats like `safetensors` which do not have the same arbitrary code execution risks as pickle-based formats.

```python
# pip install safetensors
from safetensors.torch import load_file

# Loading from a .safetensors file is secure.
model_weights = load_file("model.safetensors")
# model.load_state_dict(model_weights)
```

## How to Suppress a Finding

If you have downloaded a model file from a trusted source and have verified its integrity (e.g., by checking its hash), you can suppress this finding.

```python
import torch

# The model has been downloaded from a trusted repo and its hash was verified.
# ignore
model = torch.load("verified_model.pth")
```

Or, for this specific rule:

```python
# ignore: CSP-D204
model = torch.load("verified_model.pth")
```
