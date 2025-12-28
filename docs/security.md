# Security Analysis

CytoScnPy includes a powerful security engine powered by Rust. It performs **Secret Scanning**, **Dangerous Pattern Matching**, and **Taint Analysis**.

## Running Security Scans

To enable security checks, use the `--secrets` and `--danger` flags:

```bash
cytoscnpy . --secrets --danger
```

---

## Secret Scanning (`--secrets`)

Detects hardcoded secrets, API keys, and credentials using a combination of regex patterns and Shannon Entropy analysis.

- **AWS Keys**: Access IDs, Secret Keys.
- **API Tokens**: Stripe, Slack, PyPI, GitHub, etc.
- **Private Keys**: RSA, DSA, EC private keys.
- **High Entropy Strings**: Random strings that look like secrets but don't match specific patterns.

---

## Dangerous Code (`--danger`)

Detects patterns known to cause vulnerabilities.

For a complete list of all rules organized by category, see: **[Dangerous Code Rules](dangerous-code.md)**

---

## Taint Analysis

CytoScnPy goes beyond simple pattern matching by tracking data flow.

- **Sources**: User inputs (Flask `request`, Django `request`, CLI args, Environment variables).
- **Sinks**: Dangerous functions (SQL execute, eval, os.system).
- **Sanitizers**: Functions that clean data (e.g., `int()`, `escape()`).

If data flows from a **Source** to a **Sink** without passing through a **Sanitizer**, it is flagged as a vulnerability.

### Supported Analysis Levels

| Level               | Description                                     |
| ------------------- | ----------------------------------------------- |
| **Intraprocedural** | Checks flows within single functions.           |
| **Interprocedural** | Checks flows across functions in the same file. |
| **Cross-file**      | Checks flows across module boundaries.          |
