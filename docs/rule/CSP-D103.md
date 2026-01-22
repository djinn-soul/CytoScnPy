# CSP-D103: Cross-Site Scripting (XSS)

**Vulnerability Category:** `Injection`

**Severity:** `CRITICAL`

## Description

Cross-Site Scripting (XSS) is a type of injection vulnerability that occurs when an attacker's malicious script is injected into a trusted website. The script then executes in the victim's browser, allowing the attacker to steal sensitive information (like cookies), perform actions on behalf of the user, or manipulate the content of the web page.

This rule flags common sources of XSS in Python web frameworks, such as:

- Using `flask.Markup` or Django's `mark_safe`/`format_html` on unvalidated user input.
- Returning a `fastapi.responses.HTMLResponse` with unescaped user content.
- Disabling autoescaping in Jinja2 templates (see also [CSP-D703](./CSP-D703.md)).

## Vulnerable Code Example (Flask)

```python
from flask import Flask, request, Markup

app = Flask(__name__)

@app.route('/user')
def user_profile():
    user_name = request.args.get('name', '')

    # This is vulnerable. The user's input is rendered directly into the HTML.
    # An attacker can provide a URL like: /user?name=<script>alert('XSS')</script>
    return Markup(f"<h1>Hello, {user_name}!</h1>")

```

## Vulnerable Code Example (FastAPI)

```python
from fastapi import FastAPI
from fastapi.responses import HTMLResponse

app = FastAPI()

@app.get("/items/")
async def read_items(q: str | None = None):
    # If q contains a script, it will be executed by the browser.
    html_content = f"""
    <html>
        <body>
            <h1>Search results for: {q}</h1>
        </body>
    </html>
    """
    return HTMLResponse(content=html_content, status_code=200)
```

## Safe Code Example (Flask with Jinja2)

Web template engines like Jinja2 provide automatic escaping of variables, which is the primary defense against XSS.

```python
from flask import Flask, request, render_template_string

app = Flask(__name__)

@app.route('/user')
def user_profile():
    user_name = request.args.get('name', '')

    # Jinja2 will automatically escape the user_name variable.
    # <script> tags will be rendered as text, not executed.
    template_string = "<h1>Hello, {{ user_name }}!</h1>"
    return render_template_string(template_string, user_name=user_name)
```

## Safe Code Example (FastAPI with Jinja2)

Similarly, use a templating engine with FastAPI to ensure context-aware escaping.

```python
from fastapi import FastAPI, Request
from fastapi.responses import HTMLResponse
from fastapi.templating import Jinja2Templates

app = FastAPI()
templates = Jinja2Templates(directory="templates")

@app.get("/items/", response_class=HTMLResponse)
async def read_items(request: Request, q: str | None = None):
    # The 'q' variable will be escaped by the templating engine.
    return templates.TemplateResponse(
        request=request, name="search.html", context={"query": q}
    )
```

Where `templates/search.html` would contain:

```html
<html>
  <body>
    <h1>Search results for: {{ query }}</h1>
  </body>
</html>
```

## How to Suppress a Finding

Suppressing XSS findings is extremely dangerous. Only do so if you have manually escaped the content or are certain the input is from a trusted source and cannot be manipulated by an attacker.

```python
# The 'safe_content' variable has been sanitized or is from a trusted source.
# ignore
return Markup(safe_content)
```

Or, for this specific rule:

```python
# ignore: CSP-D103
return Markup(safe_content)
```
