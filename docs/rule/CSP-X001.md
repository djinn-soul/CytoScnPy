# CSP-X001: Generic Cross-Site Scripting (XSS)

**Vulnerability Category:** `Generic` (often detected via Taint Analysis)

**Severity:** `CRITICAL`

## Description

This rule flags potential Cross-Site Scripting (XSS) vulnerabilities that may not be tied to a specific, known framework sink (like `mark_safe` or `eval`). These generic XSS findings are typically detected through taint analysis, where untrusted input from a source (e.g., user-supplied data) is found to reach a sink (e.g., an HTML rendering context) without proper sanitization or encoding.

XSS vulnerabilities allow attackers to inject malicious client-side scripts into web pages viewed by other users. This can lead to session hijacking, data theft, defacement, or redirecting users to malicious sites.

Even in frameworks with built-in autoescaping (like Django or Flask with Jinja2), XSS can still occur if:

- Autoescaping is disabled (`autoescape=False` in Jinja2, or improper use of `mark_safe`/`|safe` filters) ([CSP-D103](./CSP-D103.md), [CSP-D703](./CSP-D703.md)).
- User input is concatenated directly into HTML strings without encoding.
- Data passes through multiple functions before rendering, and sanitization is missed along the way.

## Vulnerable Code Example

```python
from flask import Flask, request, Response
import html # For escaping, but not used here correctly

app = Flask(__name__)

@app.route('/greet')
def greet_user():
    name = request.args.get('name', 'Guest')

    # User input 'name' is directly embedded into an HTML string without escaping.
    # If name contains '<script>alert(1)</script>', it will be executed.
    html_output = f"<p>Hello, {name}!</p>"

    return Response(html_output, mimetype='text/html')
```

## Safe Code Example

The primary defense against XSS is to ensure that all data originating from untrusted sources is properly encoded or escaped before being rendered in an HTML context.

### Using a Templating Engine with Autoescaping (Recommended)

Frameworks like Flask (with Jinja2) or Django automatically escape variables by default.

```python
from flask import Flask, request, render_template_string
from jinja2 import Markup # Used to explicitly mark safe content if needed

app = Flask(__name__)

@app.route('/greet')
def greet_user():
    name = request.args.get('name', 'Guest')

    # Jinja2 will automatically escape the 'name' variable in the template.
    template = "<h1>Hello, {{ user_name }}!</h1>"
    return render_template_string(template, user_name=name)

# Example if you have a trusted HTML string:
trusted_html_snippet = "<b>User</b>"
# Use Markup to tell Jinja2 this specific string is already safe HTML.
# This is generally not needed for simple variable output.
# return render_template_string("<div>{{ trusted_content }}</div>", trusted_content=Markup(trusted_html_snippet))
```

### Manual Escaping (Less Recommended)

If you are not using a templating engine or need to build HTML strings manually, ensure you manually escape all user-provided input.

```python
import html

name = request.args.get('name', 'Guest')

# Manually escape user input before embedding it in HTML.
escaped_name = html.escape(name)

html_output = f"<p>Hello, {escaped_name}!</p>"
```

## Defense in Depth

Consider implementing a Content Security Policy (CSP) header. CSP acts as an additional layer of defense by defining which content sources are trusted, helping to prevent XSS attacks even if an injection vulnerability exists.

## How to Suppress a Finding

If you have robust, verified sanitization or encoding logic applied to the data before it reaches the rendering sink, you may suppress the finding.

```python
# The user_input has been sanitized by a trusted function before rendering.
# ignore
return f"<p>Welcome, {sanitized_input}!</p>"
```

Or, for this specific rule:

```python
# ignore: CSP-X001
return f"<p>Welcome, {user_input}!</p>" # Assuming input is safely handled elsewhere
```
