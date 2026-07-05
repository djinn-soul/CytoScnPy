# CSP-D903: Django `@csrf_exempt`

**Vulnerability Category:** `Network`

**Severity:** `HIGH`

## Description

This rule flags Django views decorated with `@csrf_exempt`. The decorator disables Django's Cross-Site Request Forgery protection for that view.

CSRF protection prevents another site from causing a user's browser to submit unwanted state-changing requests with the user's credentials. Disabling it can expose actions such as account changes, form submissions, and administrative operations.

## Vulnerable Code Example

```python
from django.views.decorators.csrf import csrf_exempt

@csrf_exempt
def update_email(request):
    request.user.email = request.POST["email"]
    request.user.save()
```

The view accepts a state-changing request without CSRF validation.

## Safe Code Example

```python
from django.views.decorators.csrf import csrf_protect

@csrf_protect
def update_email(request):
    request.user.email = request.POST["email"]
    request.user.save()
```

Keep CSRF protection enabled for browser-facing views. For APIs, use authentication mechanisms that are not vulnerable to browser credential replay, and document the threat model.

## How to Suppress a Finding

Suppress only for endpoints that cannot be reached by browsers with ambient credentials or where another explicit CSRF defense is in place.

```python
# Machine-to-machine webhook authenticated with an HMAC signature.
# ignore: CSP-D903
@csrf_exempt
def webhook(request):
    verify_signature(request)
```
