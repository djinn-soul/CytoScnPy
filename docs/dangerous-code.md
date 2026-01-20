# Dangerous Code Rules Reference

Security rules detected by the `--danger` flag, organized by category. Each category has its own detailed documentation page.

---

## Quick Access by Category

| Category                                              | Description                                            | Rule IDs |
| :---------------------------------------------------- | :----------------------------------------------------- | :------- |
| **[Code Execution](./danger/code-execution.md)**      | `eval`, `exec`, subprocess injection, insecure imports | CSP-D0xx |
| **[Injection & Logic](./danger/injection.md)**        | SQL injection, XSS, insecure XML, asserts              | CSP-D1xx |
| **[Deserialization](./danger/deserialization.md)**    | `pickle`, `yaml`, `marshal`, ML models                 | CSP-D2xx |
| **[Cryptography](./danger/cryptography.md)**          | Weak hashes, insecure PRNG, weak ciphers               | CSP-D3xx |
| **[Network & HTTP](./danger/network.md)**             | SSRF, missing timeouts, insecure `requests`            | CSP-D4xx |
| **[File Operations](./danger/filesystem.md)**         | Path traversal, zip slip, bad permissions              | CSP-D5xx |
| **[Type Safety](./danger/type-safety.md)**            | Method misuse, logic errors                            | CSP-D6xx |
| **[Best Practices](./danger/injection.md)**           | Misconfigurations, autoescape                          | CSP-D8xx |
| **[Privacy & Frameworks](./danger/modern-python.md)** | Logging sensitive data, Django secrets                 | CSP-D9xx |

---

## Severity Levels

CytoScnPy classifies findings into three severity levels:

- **CRITICAL**: Immediate risks like RCE or unauthenticated SQLi.
- **HIGH**: Risky patterns like insecure deserialization or weak crypto in production.
- **LOW**: Sub-optimal patterns, missing timeouts, or best practice violations.

Use the `--severity-threshold` flag to filter results:

```bash
cytoscnpy --danger --severity-threshold HIGH .
```
