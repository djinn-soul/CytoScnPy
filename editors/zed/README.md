# CytoScnPy for Zed

Shows [CytoScnPy](https://github.com/djinn-soul/CytoScnPy) findings in Zed:
unused code, security issues (secrets, dangerous calls, taint flows) and
quality problems in Python files.

The extension runs `cytoscnpy lsp`, the language server built into the
CytoScnPy CLI, and follows the VS Code extension: the same settings, commands
and MCP server.

It analyzes the whole workspace folder, like the VS Code extension's default
`workspace` mode, so a function another file calls is not reported as unused.
Each project in the folder (a directory with its own `pyproject.toml` or
`.cytoscnpy.toml`) is analyzed with its own configuration. A project is
analyzed when you open its first Python file, each time you save one, and
when a Python or configuration file changes on disk, and findings show for
every file in it, open or not. Zed's status bar shows the analysis while it
runs. It does not analyze as you type: while a buffer has unsaved changes,
its findings are hidden, since their positions may be wrong, and they come
back when you save or undo.

## What you get

- Diagnostics for each finding. Unused definitions are underlined at their
  name and faded; other findings cover their line.
- Quick fixes (`ctrl-.` / `cmd-.`):
  - **Remove unused …** or **Rename unused variable … to `_`**, from the
    analyzer's own fix. These are offered while the buffer matches the saved
    file; save to get them back after an edit.
  - **Suppress with `# noqa: <code>`**, which adds the code to the line's
    existing `# noqa` comment or appends a new one.
  - **Remove all unused … in this file** for each kind with several fixable
    findings, and **Remove all dead code in this file**, as in VS Code. They
    apply the analyzer's combined plan, which keeps the file valid where
    fixing each finding on its own would leave an empty block. Zed lists
    them after other servers' actions; on a line where Ruff offers many
    actions they can fall off the menu, so open it on another finding.

## Commands

The VS Code extension's commands are code actions (`ctrl-.` / `cmd-.`) on
any line without a CytoScnPy finding:

- **CytoScnPy: Analyze Current File** and **CytoScnPy: Analyze Workspace**
  run the analysis again.
- **CytoScnPy: Calculate Cyclomatic Complexity (cc)**, **Halstead Metrics
  (hal)**, **Maintainability Index (mi)** and **Raw Metrics (raw)** open the
  file's report in a new tab.

## Which checks run

Checks are configured the same way as for the CLI: the `[tool.cytoscnpy]`
table in `pyproject.toml` or a `.cytoscnpy.toml` found from the file decides
them. With no configuration, only dead code is reported. Folder exclusions
apply, and test files are skipped unless `include_tests` is set.

To override them in Zed, use the VS Code extension's settings, without the
`cytoscnpy.` prefix, under `lsp.cytoscnpy.settings` in your Zed
`settings.json`. Changes apply without restarting Zed:

```json
{
  "lsp": {
    "cytoscnpy": {
      "settings": {
        "enableSecretsScan": true,
        "enableDangerScan": true,
        "enableQualityScan": true,
        "confidenceThreshold": 80,
        "maxComplexity": 12,
        "excludeFolders": ["build"],
        "args": ["--include-tests"]
      }
    }
  }
}
```

Supported: `enableSecretsScan`, `enableDangerScan`, `enableQualityScan`,
`enableCloneScan`, `confidenceThreshold`, `excludeFolders`,
`includeFolders`, `includeTests`, `includeIpynb`, `maxComplexity`,
`minMaintainabilityIndex`, `maxNesting`, `maxArguments`, `maxLines`,
`analysisMode` (`"workspace"` or `"file"`, which analyzes each file on its
own) and `args`, extra CLI flags. A setting turned off (`false`) overrides
the project's configuration. The same settings also work in
`initialization_options`; `settings` wins where both set one.

## MCP server for the agent

Like the VS Code extension, the extension registers `cytoscnpy mcp-server`
as a context server, so Zed's agent can run CytoScnPy analyses. Enable
**cytoscnpy** in the agent panel's settings. To use your own binary, set
`context_servers.cytoscnpy.command`:

```json
{
  "context_servers": {
    "cytoscnpy": {
      "command": { "path": "/usr/local/bin/cytoscnpy", "args": ["mcp-server"] }
    }
  }
}
```

## Not available in Zed

The VS Code extension's gutter icons and its Jupyter notebook support have
no equivalent in Zed's extension API.

## The server binary

By default the extension downloads the CLI from the latest
[GitHub release](https://github.com/djinn-soul/CytoScnPy/releases) for Linux
x64, macOS (Apple Silicon and Intel) and Windows x64, and keeps it up to date.
The `lsp` subcommand first ships in v1.3.2.

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
  Analysis failures, such as an unknown flag in `args`, are logged there,
  and the first failure in a row is also shown as a notification.

## Developing the extension

Install Rust with [rustup](https://rustup.rs), then run
`zed: install dev extension` from the command palette and pick this
`editors/zed` folder. To try a local server build, run
`cargo build -p cytoscnpy-cli` at the repository root and set
`lsp.cytoscnpy.binary.path` to `target/debug/cytoscnpy-cli`.
