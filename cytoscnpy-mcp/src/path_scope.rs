//! Filesystem trust boundary for MCP path-based tools.

use std::path::{Path, PathBuf};

pub(crate) fn canonical_current_dir() -> Result<PathBuf, String> {
    std::env::current_dir()
        .and_then(|path| path.canonicalize())
        .map_err(|error| format!("Failed to establish MCP root: {error}"))
}

pub(crate) fn resolve_request_path(
    request_path: &str,
    allowed_root: &Result<PathBuf, String>,
) -> Result<PathBuf, String> {
    let root = allowed_root.as_ref().map_err(Clone::clone)?;
    let requested = Path::new(request_path);
    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        root.join(requested)
    };

    // WARNING: MCP arguments are agent-controlled; never replace this canonical
    // containment check with a lexical or existence-only path check.
    cytoscnpy::utils::validate_path_within_root(&candidate, root).map_err(|_| {
        "Access denied: requested path must exist within the configured MCP root".to_owned()
    })
}
