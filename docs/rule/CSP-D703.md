# CSP-D703: Jinja2 Autoescaping Disabled

**Vulnerability Category:** `Best Practices`

**Severity:** `HIGH`

## Description

This rule flags Jinja2 environments that have autoescaping disabled (`autoescape=False`). Jinja2 is a popular templating engine for Python. By default, it automatically escapes special HTML characters in variables rendered into templates. This behavior is crucial for preventing Cross-Site Scripting (XSS) attacks, as it ensures that any user-provided HTML or JavaScript is rendered as literal text rather than being executed by the browser.

Disabling autoescaping (`autoescape=False`) means that all variables rendered into the template will be inserted verbatim. If any of these variables contain untrusted user input, it can lead to an XSS vulnerability.

## Vulnerable Code Example

```python
from jinja2 import Environment, FileSystemLoader, Template

# Disabling autoescaping for the entire environment.
# This is a dangerous practice if any of the templates render user input.
env = Environment(loader=FileSystemLoader('/templates'), autoescape=False)

# Assume 'user_comment' comes from user input.
user_comment = "<script>alert('XSS attack!')</script>"

# If this template is rendered, the script will execute in the user's browser.
template = env.from_string("<div>{{ comment }}</div>")
rendered_html = template.render(comment=user_comment)
```

## Safe Code Example

Ensure that autoescaping is enabled for your Jinja2 environment, or explicitly enable it for specific templates or variables.

### Enabling Autoescaping by Default

The recommended approach is to set `autoescape=True` when creating the Jinja2 `Environment`. You can also specify which file extensions should be autoescaped.

```python
from jinja2 import Environment, FileSystemLoader

# Autoescaping is enabled by default for HTML, XML, and CSS.
# You can explicitly set it or rely on Jinja2's defaults.
env = Environment(
    loader=FileSystemLoader('/templates'),
    autoescape=True # Or rely on Jinja2's default which is True for .html
)

user_comment = "<script>alert('XSS attack!')</script>"

# Even though autoescape is on, Jinja2 will automatically escape `user_comment`.
template = env.from_string("<div>{{ comment }}</div>")
rendered_html = template.render(comment=user_comment)
# The output will be: <div>&lt;script&gt;alert('XSS attack!')&lt;/script&gt;</div>
```

### Explicitly Escaping or Marking Safe

If you *must* disable autoescaping for a specific template or need to render HTML content that you've safely escaped or generated, you can control it more granularly.

```python
from jinja2 import Environment, Template

# If you MUST disable autoescape for a specific template:
# NEVER do this with user-controlled data.
template_string = "<div>{{ comment }}</div>"
template = Template(template_string, autoescape=False) # Explicitly disabled for this template

# If you have trusted HTML that you want to render UNSAFELY (use with extreme caution):
from jinja2 import Markup
trusted_html = "<b>This is safe HTML</b>"
rendered_unsafe = template.render(comment=Markup(trusted_html)) # Mark as safe

# If you have user input that you have securely sanitized (e.g. with bleach)
# and want to render it as HTML:
# import bleach
# safe_user_html = bleach.clean(user_input_html)
# rendered_safe_user_html = template.render(comment=Markup(safe_user_html))
```

## How to Suppress a Finding

If you have explicitly disabled autoescaping for a specific reason (e.g., you are rendering pure, trusted HTML or have sanitized the input using a library like `bleach` and are marking it safe), you can suppress this finding.

```python
from jinja2 import Environment, Markup

# The environment has autoescape=False, but the specific content is trusted.
# ignore
env = Environment(autoescape=False)
safe_html_content = Markup("<p>This is trusted HTML.</p>")
template = env.from_string("<div>{{ content }}</div>")
rendered = template.render(content=safe_html_content)
```

Or, for this specific rule:

```python
# ignore: CSP-D703
env = Environment(autoescape=False)
```
