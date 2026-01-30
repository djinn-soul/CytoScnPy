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

### Secret Scanning Configuration

Tune detection via `.cytoscnpy.toml` or `pyproject.toml`:

```toml
[cytoscnpy.secrets_config]
entropy_threshold = 4.5
min_length = 16
entropy_enabled = true
scan_comments = true
skip_docstrings = false
min_score = 50
suspicious_names = ["db_password", "oauth_token"]

[[cytoscnpy.secrets_config.patterns]]
name = "Slack Token"
regex = "xox[baprs]-([0-9a-zA-Z]{10,48})"
severity = "HIGH"
```

---

## Dangerous Code (`--danger`)

Detects patterns known to cause vulnerabilities.

For a complete list of all rules organized by category, see: **[Dangerous Code Rules](dangerous-code.md)**

### Rule Categories

- [Code Execution & Unsafe Calls](danger/code-execution.md) (CSP-D0xx)
- [Injection & Logic Attacks](danger/injection.md) (CSP-D1xx)
- [Deserialization](danger/deserialization.md) (CSP-D2xx)
- [Cryptography & Randomness](danger/cryptography.md) (CSP-D3xx)
- [Network & HTTP Security](danger/network.md) (CSP-D4xx)
- [File Operations & Path Traversal](danger/filesystem.md) (CSP-D5xx)
- [Modern Python & Frameworks](danger/modern-python.md) (CSP-D9xx)

## Rule Count

The current danger rule set includes 31 rules.

### Danger + Taint Configuration

Use `danger_config` to control taint analysis and filtering:

```toml
[cytoscnpy.danger_config]
enable_taint = true
severity_threshold = "LOW" # LOW, MEDIUM, HIGH, CRITICAL
excluded_rules = ["CSP-D101"]
custom_sources = ["mylib.get_input"]
custom_sinks = ["mylib.exec"]
```

---

## Taint Analysis

CytoScnPy goes beyond simple pattern matching by tracking data flow.

- **Sources**: User inputs (Flask `request`, Django `request`, CLI args, Environment variables).
- **Sinks**: Dangerous functions (SQL execute, eval, os.system).
- **Sanitizers**: Functions that clean data (e.g., `int()`, `escape()`).

If data flows from a **Source** to a **Sink** without passing through a **Sanitizer**, it is flagged as a vulnerability.

### Supported Analysis Levels

| Level               | Description                                     | Implementation                    |
| ------------------- | ----------------------------------------------- | --------------------------------- |
| **Intraprocedural** | Checks flows within single functions.           | Fast, catches local bugs          |
| **Interprocedural** | Checks flows across functions in the same file. | Tracks cross-function data flow   |
| **Cross-file**      | Checks flows across module boundaries.          | Deep analysis (highest precision) |

CytoScnPy uses a multi-layered approach to track taint across your entire project, identifying vulnerabilities where untrusted input reaches critical system sinks.
