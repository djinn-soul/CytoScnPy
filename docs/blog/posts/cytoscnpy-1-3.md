---
date: 2026-11-06
authors:
  - djinn09
categories:
  - Releases
---

# CytoScnPy 1.3: Whole-Repository Assessment and Verified Fixes

CytoScnPy 1.3.0 grew from a dead-code and security analyzer into a whole-repository assessment tool, and 1.3.1 makes its fixes safe to apply anywhere, notebooks and monorepos included.

<!-- more -->

## 1.3.0: `deslop` and a Python API

- **`cytoscnpy deslop`** scores a repository with a 0–100 Slop Index across 10 weighted dimensions, lists prioritized fixes, and enforces CI gates. See [DeSlop: LLM Friction Score](../../deslop.md).
- **A Python API**: `scan`, `scan_code`, `scan_json`, and `scan_code_json` return structured results in-process. See [Python API & pytest](../../python-api.md).
- **New security rules** for LDAP and XPath injection, file permissions, TOCTOU races, and PyNaCl misuse.
- **Rule-aware suppressions**: `# noqa: <code>` hides only the rule it names.
- **Faster scans**: a 10k-file corpus went from 5.7s to 3.4s with identical output.

## 1.3.1: Verified Fixes, Notebooks, and Monorepos

- **Verified fixes**: every suggested dead-code fix is checked to leave valid Python before it is offered, in the CLI, JSON output, and VS Code quick fixes.
- **Notebooks**: with `--include-ipynb --ipynb-cells`, findings and fixes point at the right cell, and `--fix` works on notebooks.
- **Monorepos**: `cytoscnpy projects` lists nested projects, and the VS Code extension scans each with its own configuration.
- **Settings that override both ways**: new `--no-*` flags turn off what project configuration turns on, and VS Code settings set to `false` now do the same.
- **Fewer false positives**: forward-reference annotations, registries, external base classes, and module-attribute calls are now understood.
- **HTML report**: light and dark themes, issue cards with copyable `# noqa` comments, and a Slop Index page from `deslop --html`.

## Upgrading

```bash
pip install --upgrade cytoscnpy
```

Update your pre-commit hook `rev` to `v1.3.1`, and see the [migration guide](../../migration.md) for behavior changes. Full release notes are on [GitHub](https://github.com/djinn-soul/CytoScnPy/releases/tag/v1.3.1).
