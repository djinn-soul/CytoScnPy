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

### Secret Rules

| Rule ID | Detects | Severity |
|---|---|---|
| `CSP-S101` | AWS access key | HIGH |
| `CSP-S102` | AWS secret key | CRITICAL |
| `CSP-S103` | Generic API key | HIGH |
| `CSP-S104` | GitHub token | CRITICAL |
| `CSP-S105` | GitHub OAuth token | CRITICAL |
| `CSP-S106` | GitHub App token | CRITICAL |
| `CSP-S107` | GitLab personal access token | CRITICAL |
| `CSP-S108` | Slack bot token | HIGH |
| `CSP-S109` | Slack user token | HIGH |
| `CSP-S110` | Stripe live key | CRITICAL |
| `CSP-S111` | Stripe test key | MEDIUM |
| `CSP-S112` | Private key | CRITICAL |
| `CSP-S113` | Google API key | HIGH |
| `CSP-S114` | Heroku API key | HIGH |
| `CSP-S115` | SendGrid API key | HIGH |
| `CSP-S116` | Twilio API key | HIGH |
| `CSP-S117` | npm token | HIGH |
| `CSP-S118` | PyPI token | HIGH |
| `CSP-S119` | Discord token | HIGH |
| `CSP-S120` | Database connection string with credentials | CRITICAL |
| `CSP-S121` | Anthropic API key | CRITICAL |
| `CSP-S122` | JWT secret | CRITICAL |
| [`CSP-S200`](rule/CSP-S200.md) | High-entropy string that looks like a secret | HIGH |
| [`CSP-S300`](rule/CSP-S300.md) | Hardcoded value assigned to a suspicious name (`password`, `token`, ...) | varies |
| `CSP-CUSTOM-<name>` | A custom pattern from `secrets_config.patterns` without its own `rule_id` | as configured |

`CSP-S101`–`CSP-S122` are the built-in pattern detectors. Suppress one with its
own ID, for example `# noqa: CSP-S104`.

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
rule_id = "CSP-S900"  # optional; defaults to CSP-CUSTOM-<name with dashes>
```

| Key | Default | Meaning |
|---|---|---|
| `entropy_threshold` | `4.5` | Minimum Shannon entropy (0.0–8.0) for `CSP-S200` |
| `min_length` | `16` | Shortest string checked for entropy |
| `entropy_enabled` | `true` | Turn the entropy detector on or off |
| `scan_comments` | `true` | Also scan comments |
| `skip_docstrings` | `false` | Skip docstrings in entropy scanning |
| `min_score` | `50` | Minimum confidence (0–100) to report |
| `suspicious_names` | `[]` | Extra variable names for `CSP-S300`, added to the built-in list |
| `patterns` | `[]` | Custom detectors: `name`, `regex`, `severity` (default `HIGH`), optional `rule_id` |

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
- [Type Safety](danger/type-safety.md) (CSP-D6xx)
- [Best Practices](danger/best-practices.md) (CSP-D7xx)
- [Open Redirect](danger/open-redirect.md) (CSP-D8xx)
- [Privacy](danger/privacy.md) and [Modern Python & Frameworks](danger/modern-python.md) (CSP-D9xx)

## Rule Count

The current security rule set includes:

- 53 dangerous code rules (`CSP-D*`)
- 24 secret rules (`CSP-S101`–`CSP-S122`, `CSP-S200`, `CSP-S300`), plus any custom patterns

### Danger + Taint Configuration

Use `danger_config` to control taint analysis and filtering:

```toml
[cytoscnpy.danger_config]
enable_taint = true
severity_threshold = "LOW" # LOW, MEDIUM, HIGH, CRITICAL
excluded_rules = ["CSP-D101"]
custom_sources = ["mylib.get_input"]
custom_sinks = ["mylib.exec"]

# Sanitizers are scoped by vulnerability type and behavior.
[cytoscnpy.danger_config.sanitizers.ssrf]
return_value = ["validate_allowed_url"]
guard = ["is_allowed_url"]
side_effect = ["validate_url_or_raise"]

[cytoscnpy.danger_config.sanitizers.path_traversal]
return_value = ["safe_join"]

[cytoscnpy.danger_config.sanitizers.sql_injection]
return_value = ["build_parameterized_query"]

[cytoscnpy.danger_config.sanitizers.command_injection]
return_value = ["quote_shell_argument"]

[cytoscnpy.danger_config.sanitizers.code_injection]  # eval, exec, compile
guard = ["is_trusted_expression"]
```

The sanitizer groups are `ssrf`, `path_traversal`, `sql_injection`,
`command_injection`, and `code_injection`. The older top-level
`custom_sanitizers = ["clean"]` list still works: it treats each function as a
`return_value` sanitizer for every vulnerability type. Prefer the scoped groups,
which cannot hide unrelated findings.

Sanitizers are never inferred from names such as `validated_url`. `return_value`
applies only to the returned value, `guard` only to the truthy branch, and
`side_effect` only after the call returns. Each sanitizer affects only its
configured vulnerability type.

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
