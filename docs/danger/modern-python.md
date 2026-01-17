# Category 9 & 10: Modern Python & Framework Security

This category covers security rules for modern Python features (introduced in 2025/2026), ML/AI workflows, and specific web frameworks like Django and Flask.

| Rule ID      | Pattern                                         | Severity     | Why it's risky                      | Safer alternative / Fix                        |
| :----------- | :---------------------------------------------- | :----------- | :---------------------------------- | :--------------------------------------------- |
| **CSP-D901** | `asyncio.create_subprocess_shell` (dynamic)     | **CRITICAL** | Async command injection             | Use `create_subprocess_exec` with list args    |
| **CSP-D901** | `os.popen(dynamic)`                             | **HIGH**     | Legacy command injection            | Use `subprocess.run` with list args            |
| **CSP-D901** | `pty.spawn(dynamic)`                            | **HIGH**     | PTY command injection               | Validate/allowlist commands                    |
| **CSP-D902** | `torch.load()` / `joblib.load()` / `keras.load` | **CRITICAL** | Arbitrary code execution via pickle | Use `weights_only=True` or `torch.safe_load()` |
| **CSP-D903** | Logging sensitive variables                     | **MEDIUM**   | Data leakage in logs                | Redact passwords, tokens, API keys             |
| **CSP-D904** | Hardcoded `SECRET_KEY`                          | **CRITICAL** | Key exposure in Django              | Store in environment variables                 |
| **CSP-D601** | Type-based method misuse                        | **HIGH**     | Logic errors / Type confusion       | Use static typing and validation               |

## In-depth: ML Model Deserialization (CSP-D902)

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

## In-depth: Logging Sensitive Data (CSP-D903)

Logging sensitive information like API keys or user passwords can lead to data breaches if logs are compromised.

### Dangerous Pattern

```python
import logging
api_key = "sk-..."
logging.info(f"Using API key: {api_key}") # DANGEROUS: Leaks in logs
```

### Safe Alternative

```python
import logging
api_key = "sk-..."
logging.info("Using API key: [REDACTED]") # SAFE
```
