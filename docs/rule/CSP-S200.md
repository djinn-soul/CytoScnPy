# CSP-S200: Secret Pattern Match

**Category:** `Secrets`

**Severity:** `HIGH`

## Description

This rule reports high-entropy string literals: random-looking values such as API keys and tokens that do not match a known provider format. The entropy threshold, minimum length, and comment and docstring scanning are set in `secrets_config`.

Secrets that match a known provider format are reported under their own IDs, `CSP-S101` to `CSP-S122` (AWS, GitHub, Stripe, Slack, Anthropic, and others). See the [Secret Rules table](../security.md#secret-rules) for the full list.

## Trigger Example

```python
GITHUB_TOKEN = "ghp_1234567890abcdefghijklmnopqrstuvwxyz"
```

## Recommended Refactor

```python
import os

GITHUB_TOKEN = os.environ["GITHUB_TOKEN"]
```

## Notes

- UUIDs, file paths, and URLs are not reported, even when their entropy is high.
- Custom patterns can be added in `secrets_config.patterns`.

## How to Suppress a Finding

```python
# ignore
# noqa: CSP-S200
```
