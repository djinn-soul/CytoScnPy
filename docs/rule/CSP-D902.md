# CSP-D902: Hardcoded Django `SECRET_KEY`

**Vulnerability Category:** `Privacy`

**Severity:** `CRITICAL`

## Description

This rule flags the hardcoding of Django's `SECRET_KEY` directly within the source code. The `SECRET_KEY` is a crucial security setting for Django projects. It is used to cryptographically sign information, like session cookies, and is essential for ensuring the integrity and authenticity of data.

If the `SECRET_KEY` is hardcoded and committed to a version control system (like Git), it becomes exposed to anyone who can access the codebase. An attacker who obtains the `SECRET_KEY` can:
- **Forge session cookies:** Impersonate logged-in users.
- **Tamper with data:** Modify signed data, such as password reset tokens.
- **Potentially compromise the entire application:** Depending on how the secret is used, it might be possible to gain unauthorized access or execute malicious code.

## Vulnerable Code Example

```python
# settings.py
SECRET_KEY = 'django-insecure-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'
# ... other settings ...
```
Committing this `settings.py` file to a public repository or sharing it would expose the `SECRET_KEY`.

## Safe Code Example

The `SECRET_KEY` should never be hardcoded. Instead, it should be loaded from environment variables or a secure configuration management system. This ensures the secret is not present in the codebase and can be managed securely in different environments (development, staging, production).

### Using Environment Variables

**1. In `settings.py`:**
```python
import os

# Load the secret key from an environment variable.
# Provide a default for local development if the variable is not set.
SECRET_KEY = os.environ.get('DJANGO_SECRET_KEY', 'django-insecure-default-for-local-dev-only')

# If DJANGO_SECRET_KEY is not set, Django will issue a warning.
# For production, it's essential that DJANGO_SECRET_KEY is set externally.
```

**2. Setting the environment variable:**

*   **On Linux/macOS (in your shell):**
    ```bash
    export DJANGO_SECRET_KEY='your-super-secret-key-for-production'
    python manage.py runserver
    ```
*   **On Windows (cmd):**
    ```cmd
    set DJANGO_SECRET_KEY=your-super-secret-key-for-production
    python manage.py runserver
    ```
*   **Using `.env` files (with `python-dotenv`):**
    Install: `pip install python-dotenv`
    Create a `.env` file in your project root:
    ```
    DJANGO_SECRET_KEY=your-super-secret-key-for-production
    ```
    In your `settings.py` or a startup script:
    ```python
    from dotenv import load_dotenv
    load_dotenv() # Load variables from .env file
    # ... then os.environ.get('DJANGO_SECRET_KEY', ...) will work
    ```

## How to Suppress a Finding

You should never hardcode your `SECRET_KEY`. If this finding appears, it means you have a vulnerability that needs immediate attention. Suppressing this finding is highly discouraged and should only be done if you've already moved the secret to an environment variable and the tool is incorrectly flagging an old file or a temporary state.

```python
# This is a placeholder for local development ONLY and is NOT committed to VCS.
# The actual secret is loaded from environment variables in production.
# ignore
SECRET_KEY = 'django-insecure-default-for-local-dev-only'
```

Or, for this specific rule:

```python
# ignore: CSP-D902
SECRET_KEY = 'hardcoded-for-testing-only'
```
