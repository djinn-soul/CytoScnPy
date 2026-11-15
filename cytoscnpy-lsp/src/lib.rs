//! Language Server Protocol server for `CytoScnPy`.
//!
//! `cytoscnpy lsp` speaks LSP over stdio, so any editor with an LSP client
//! (Zed, Neovim, Helix, Emacs, Sublime Text, ...) shows CytoScnPy findings.
//! Like the VS Code extension, it analyzes the workspace folder holding a
//! Python file when the file is opened and each time one is saved, by running
//! the same analysis as `cytoscnpy <folder> --json`, so the project's
//! `pyproject.toml` or `.cytoscnpy.toml` decides which checks run and code
//! used from another file is not reported as unused. Files outside every
//! workspace folder, or all files with `{"analysisMode": "file"}`, are
//! analyzed alone, as `cytoscnpy <file> --json --force-exclude` does.
//!
//! Each finding becomes a diagnostic. Code actions remove or rename unused
//! code, add `# noqa` comments, and, like the VS Code extension's "Fix All"
//! actions, remove every unused finding of a kind or all dead code in a file.
//!
//! Also as in VS Code: each nested project is analyzed with its own
//! configuration, files changed on disk are re-analyzed, the extension's
//! settings map to CLI flags, its commands are LSP
//! commands and source actions, and analysis reports progress.

mod actions;
mod analysis;
mod analyze;
mod client;
mod commands;
mod diagnostics;
mod position;
mod project;
mod report;
mod server;
mod settings;
mod uri;
mod workspace;

pub use server::run;

use lsp_server::Connection;

/// Serves LSP over stdin and stdout until the client shuts the server down.
///
/// # Errors
///
/// Returns an error when the connection fails or the client breaks the
/// protocol.
pub fn run_stdio() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (connection, io_threads) = Connection::stdio();
    run(&connection)?;
    // Dropping the connection closes the writer so its thread can finish.
    drop(connection);
    io_threads.join()?;
    Ok(())
}
