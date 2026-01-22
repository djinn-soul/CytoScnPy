# CSP-D403: Debug Mode Enabled in Production

**Vulnerability Category:** `Network`

**Severity:** `HIGH`

## Description

This rule flags when a web application is run with debug mode enabled. Debug mode is a powerful feature in web frameworks like Flask, Django, and others, providing detailed error pages and often an interactive debugger in the browser.

While extremely useful during development, running an application in debug mode in a production environment is a major security risk.
- **Information Leakage:** Detailed traceback pages can reveal sensitive information about your application's structure, libraries, file paths, and configuration.
- **Remote Code Execution (RCE):** Some frameworks (notably Flask with the Werkzeug debugger) provide an interactive web-based console on the error page. This console is often protected by a PIN, but if an attacker can guess or bypass the PIN, they can execute arbitrary Python code on the server, leading to a full system compromise.

Debug mode should never be enabled in a production deployment.

## Vulnerable Code Example (Flask)

```python
from flask import Flask

app = Flask(__name__)

@app.route('/')
def hello():
    # This will cause an error
    1 / 0
    return "Hello, World!"

if __name__ == '__main__':
    # Running with debug=True exposes the interactive debugger
    app.run(debug=True)
```

## Vulnerable Configuration (Django)

In a Django `settings.py` file:
```python
# This should always be False in production
DEBUG = True
```

## Safe Configuration

Debug mode should be controlled by an environment variable or a configuration file that is different for development and production environments.

### Safe Flask Example

```python
from flask import Flask
import os

app = Flask(__name__)

# It's better to use a production-ready WSGI server like Gunicorn or uWSGI
# instead of app.run() in production.
# If you must use app.run(), ensure debug is disabled.
if __name__ == '__main__':
    # Get debug status from an environment variable, defaulting to False
    is_debug = os.environ.get("FLASK_DEBUG", "false").lower() == "true"
    app.run(debug=is_debug)
```

### Safe Django `settings.py`

```python
import os

# Set DEBUG to False by default, and only enable it if an env var is explicitly set.
# The `os.environ.get(...) != 'False'` is a robust way to handle this, as most
# non-empty strings are True in a boolean context.
DEBUG = os.environ.get('DJANGO_DEBUG', 'False') == 'True'
```

The standard practice is to use a production-grade WSGI server (like Gunicorn, uWSGI, or Daphne) to run your application, rather than the built-in development servers provided by the frameworks. These servers do not use the framework's debug mode.

## How to Suppress a Finding

You should not suppress this finding for code running in a production environment. If this code is part of a development-only script, you can add a suppression comment.

```python
# This is a development startup script and will not be used in production.
# ignore
app.run(debug=True)
```

Or, for this specific rule:

```python
# ignore: CSP-D403
app.run(debug=True)
```
