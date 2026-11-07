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
//! code and add `# noqa` comments.

mod actions;
mod analysis;
mod diagnostics;
mod position;
mod project;
mod report;
mod server;
mod uri;

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
