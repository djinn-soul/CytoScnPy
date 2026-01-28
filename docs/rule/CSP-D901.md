# CSP-D901: Logging of Sensitive Data

**Vulnerability Category:** `Privacy`

**Severity:** `MEDIUM`

## Description

This rule flags instances where sensitive information is logged to console output or log files. Logging sensitive data, such as passwords, API keys, session tokens, Personally Identifiable Information (PII), or financial details, can lead to data breaches if log files are compromised or accessed by unauthorized personnel.

Even if logs are generally secured, accidental exposure or misconfiguration can expose this data. It is a best practice to avoid logging sensitive information altogether.

## Vulnerable Code Example

```python
import logging
import secrets

# Setup logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

user_id = 12345
api_key = secrets.token_hex(16) # Generate a random API key
password = "thisIsASecurePassword123!" # A real password should never be used directly

# Logging sensitive information directly
logger.info(f"User ID: {user_id}, API Key: {api_key}, Password: {password}")
logger.warning("Processing sensitive data for user %s", user_id)
```
In this example, the `api_key` and `password` are logged, which is a security risk.

## Safe Code Example

Sensitive information should never be logged directly. If you need to log information about a user or a transaction, redact or mask the sensitive fields.

```python
import logging
import secrets

# Setup logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

user_id = 12345
api_key = secrets.token_hex(16)
password_plaintext = "thisIsASecurePassword123!" # Never log the plaintext password

# Masking sensitive data before logging
# Redact the API Key and Password
masked_api_key = api_key[:4] + "****" + api_key[-4:]
masked_password = "*" * len(password_plaintext)

logger.info(f"User ID: {user_id}, API Key: {masked_api_key}, Password: {masked_password}")
logger.warning("Processing sensitive data for user %s", user_id)
```

## Best Practices for Logging Sensitive Data

-   **Avoid Logging:** The best approach is to not log sensitive data at all. If you must log information related to a sensitive field, log only what is necessary for debugging or auditing, and ensure it's anonymized or masked.
-   **Redact/Mask:** Replace sensitive parts of the data with placeholders like `****` or a fixed string (e.g., `[REDACTED]`).
-   **Hash (for secrets):** If you need to log an identifier related to a secret (e.g., the first 4 and last 4 characters of an API key), hash the full secret first, then log the hash along with the non-sensitive parts of the identifier.
-   **Configuration:** Use configuration settings to control logging levels and what information is logged, allowing sensitive logging to be turned off in production.
-   **Log Rotation and Access Control:** Ensure that log files themselves are stored securely with appropriate access controls and rotation policies.

## How to Suppress a Finding

If you have implemented robust sanitization or masking for the sensitive data before it's logged, or if the logging is part of a secure, internal audit mechanism that is strictly controlled, you may suppress this finding.

```python
# The API key is masked before logging.
# ignore
logger.info(f"API Key used: {mask_sensitive_data(api_key)}")
```

Or, for this specific rule:

```python
# ignore: CSP-D901
logger.info("Password processed: %s", "[REDACTED]")
```
