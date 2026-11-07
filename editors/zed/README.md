# CytoScnPy for Zed

Shows [CytoScnPy](https://github.com/djinn-soul/CytoScnPy) findings in Zed:
unused code, security issues (secrets, dangerous calls, taint flows) and
quality problems in Python files.

The extension runs `cytoscnpy lsp`, the language server built into the
CytoScnPy CLI. Like the VS Code extension, it analyzes a file when you open it
and each time you save it. It does not analyze as you type.

## What you get

- Diagnostics for each finding. Unused definitions are underlined at their
  name and faded; other findings cover their line.
- Quick fixes (`ctrl-.` / `cmd-.`):
  - **Remove unused …** or **Rename unused variable … to `_`**, from the
    analyzer's own fix. These are offered while the buffer matches the saved
    file; save to get them back after an edit.
  - **Suppress with `# noqa: <code>`**, which adds the code to the line's
    existing `# noqa` comment or appends a new one.

## Which checks run

Checks are configured the same way as for the CLI: the `[tool.cytoscnpy]`
table in `pyproject.toml` or a `.cytoscnpy.toml` found from the file decides
them. With no configuration, only dead code is reported. Folder exclusions
apply, and test files are skipped unless `include_tests` is set.

To add CLI flags in Zed only, pass them as `args` in the initialization
options in your Zed `settings.json`:

```json
{
  "lsp": {
    "cytoscnpy": {
      "initialization_options": {
        "args": ["--secrets", "--danger", "--quality"]
      }
    }
  }
}
```

## The server binary

By default the extension downloads the CLI from the latest
[GitHub release](https://github.com/djinn-soul/CytoScnPy/releases) for Linux
x64, macOS (Apple Silicon and Intel) and Windows x64, and keeps it up to date.
The `lsp` subcommand first ships in the release after v1.3.1.

To use a binary you installed yourself (for example with `install.sh`, or on
another platform), point the extension at it:

```json
{
  "lsp": {
    "cytoscnpy": {
      "binary": {
        "path": "/usr/local/bin/cytoscnpy",
        "arguments": ["lsp"]
      }
    }
  }
}
```

Use the native CLI binary. The `cytoscnpy` command that `pip install
cytoscnpy` provides is a Python wrapper, and it cannot run the language
server (or `mcp-server`).

## Troubleshooting

- **No diagnostics.** Zed may not start extension servers for Python unless
  they are listed. Add CytoScnPy to the Python language servers:

  ```json
  {
    "languages": {
      "Python": {
        "language_servers": ["cytoscnpy", "..."]
      }
    }
  }
  ```

  `"..."` keeps the servers Zed enables by default.
- **Server errors.** Run `zed: open language server logs` (`dev: open
  language server logs` on older Zed versions) and choose `cytoscnpy`.
  Analysis failures, such as an unknown flag in `args`, are logged there.

## Developing the extension

Install Rust with [rustup](https://rustup.rs), then run
`zed: install dev extension` from the command palette and pick this
`editors/zed` folder. To try a local server build, run
`cargo build -p cytoscnpy-cli` at the repository root and set
`lsp.cytoscnpy.binary.path` to `target/debug/cytoscnpy-cli`.
