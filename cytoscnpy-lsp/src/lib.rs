//! Language Server Protocol server for `CytoScnPy`.
//!
//! `cytoscnpy lsp` speaks LSP over stdio, so any editor with an LSP client
//! (Zed, Neovim, Helix, Emacs, Sublime Text, ...) shows CytoScnPy findings.
//! Like the VS Code extension, it analyzes a Python file when it is opened
//! and each time it is saved, by running the same analysis as
//! `cytoscnpy <file> --json --force-exclude`, so the project's
//! `pyproject.toml` or `.cytoscnpy.toml` decides which checks run.
//!
//! Each finding becomes a diagnostic. Code actions remove or rename unused
//! code and add `# noqa` comments.

mod actions;
mod analysis;
mod diagnostics;
mod position;
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
