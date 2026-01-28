# Category 9: Information Privacy & Frameworks (CSP-D9xx)

This category covers security rules for sensitive data handling, information leakage, and specific web framework misconfigurations (e.g., Django, Flask).

| Rule ID      | Pattern                     | Severity     | Why it's risky         | Safer alternative / Fix            |
| :----------- | :-------------------------- | :----------- | :--------------------- | :--------------------------------- |
| **CSP-D901** | Logging sensitive variables | **MEDIUM**   | Data leakage in logs   | Redact passwords, tokens, API keys |
| **CSP-D902** | Hardcoded `SECRET_KEY`      | **CRITICAL** | Key exposure in Django | Store in environment variables     |

## In-depth: Logging Sensitive Data (CSP-D901)

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

## In-depth: Framework Secrets (CSP-D902)

Framework settings files often contain sensitive keys that must not be committed to source control. Hardcoded secrets are easily discovered by attackers.

### Dangerous Pattern (Django)

```python
# settings.py
SECRET_KEY = 'django-insecure-hardcoded-key-here' # VULNERABLE
```

### Safe Alternative

```python
import os
SECRET_KEY = os.environ.get('DJANGO_SECRET_KEY') # SAFE: Loaded from env
```
