# Pre-commit Hooks

CytoScnPy provides several pre-commit hooks to automate code analysis in your local development workflow. This ensures that unused code, security vulnerabilities, and quality issues are caught before they reach your repository.

## Installation

1.  **Install pre-commit**:

    ```bash
    pip install pre-commit
    ```

2.  **Add Configuration**: Create or update `.pre-commit-config.yaml` in your project root:

    ```yaml
    repos:
      - repo: https://github.com/djinn09/CytoScnPy
        rev: v1.2.1 # Use the latest release tag
        hooks:
          - id: cytoscnpy-check
            # Optional: custom arguments
            # args: ['--confidence', '60', '--danger', '--quality']
    ```

3.  **Install Hooks**:
    ```bash
    pre-commit install
    ```

## Available Hooks

| Hook ID              | Description                                    | Recommended For           |
| :------------------- | :--------------------------------------------- | :------------------------ |
| `cytoscnpy-check`    | Full analysis (security + quality + dead code) | General protection        |
| `cytoscnpy-danger`   | Scans for dangerous patterns (SQLi, XSS, etc.) | Security-focused projects |
| `cytoscnpy-secrets`  | Scans for hardcoded credentials/API keys       | All projects              |
| `cytoscnpy-quality`  | Checks CC, MI, and unused code                 | Maintaining code health   |
| `cytoscnpy-security` | `cytoscnpy-danger` + `cytoscnpy-secrets`       | Security hardening        |

## Usage & Best Practices

### Selective Analysis

If you only want to fail on security issues but want to see quality warnings, use separate hooks:

```yaml
- id: cytoscnpy-security
  args: ["--fail-threshold", "0"]
- id: cytoscnpy-quality
  args: ["--fail-on-quality", "false"]
```

### Strictness Levels

You can enforce strict quality gates using these flags in `args`:

- `--fail-on-quality`: Exit with code 1 if any quality issues are found.
- `--fail-threshold <N>`: Fail if unused code percentage exceeds N.
- `--max-complexity <N>`: Fail if any function exceeds complexity N.

### Performance

CytoScnPy is built in Rust and is designed to be extremely fast. However, for very large monorepos, you may want to limit the frequency:

```yaml
- id: cytoscnpy-check
  stages: [push] # Only run on push instead of every commit
```

## Troubleshooting

### "Too many open files"

If running on thousands of files at once, you might hit OS limits. You can limit the hook to specific directories:

```yaml
hooks:
  - id: cytoscnpy-check
    files: ^src/
```

### Suppression

To ignore a specific finding on a line, use any of these formats:

```python
def legacy_function():  # noqa
    pass  # bare noqa suppresses all

unused_var = 1  # ignore
    # bare ignore also suppresses all

secret_key = "..."  # pragma: no cytoscnpy
    # legacy format, still supported

other_var = value  # noqa: E501, CSP
    # mixed codes: suppresses CytoScnPy because CSP is in the list
```

> [!NOTE]
> Inline suppression comments (`# noqa`, `# ignore`, `# pragma: no cytoscnpy`) apply to dead code, security, quality, and clone findings on a specific line. For ignoring rules across the entire project, use the `ignore` list in your `.cytoscnpy.toml` configuration file.
