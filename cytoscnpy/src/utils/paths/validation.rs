//! Validate source and output paths against traversal.

/// Validates that a path is contained within an allowed root directory.
///
/// This provides defense-in-depth against path traversal vulnerabilities.
///
/// # Errors
///
/// Returns an error if the path or root cannot be canonicalized,
/// or if the path lies outside the root.
pub fn validate_path_within_root(
    path: &std::path::Path,
    root: &std::path::Path,
) -> anyhow::Result<std::path::PathBuf> {
    let canonical_path = path
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Failed to resolve path {}: {}", path.display(), e))?;
    let canonical_root = root
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Failed to resolve root {}: {}", root.display(), e))?;

    if canonical_path.starts_with(&canonical_root) {
        Ok(canonical_path)
    } else {
        anyhow::bail!(
            "Path traversal detected: {} is outside of {}",
            path.display(),
            root.display()
        )
    }
}

/// Validates that an output path doesn't escape via traversal.
///
/// This ensures that the path stays within the allowed root directory.
/// When `root` is `Some`, uses that as the containment boundary.
/// When `root` is `None`, falls back to the current working directory (CWD).
///
/// It resolves the longest existing ancestor to handle symlinks and checks
/// that the remaining path components do not contain `..` (`ParentDir`).
///
/// # Errors
///
/// Returns an error if:
/// - The root directory cannot be determined or resolved.
/// - The path traverses outside the allowed root.
/// - The path contains `..` components in the non-existent portion.
pub fn validate_output_path(
    path: &std::path::Path,
    root: Option<&std::path::Path>,
) -> anyhow::Result<std::path::PathBuf> {
    let canonical_root = if let Some(r) = root {
        r.canonicalize().map_err(|e| {
            anyhow::anyhow!(
                "Failed to canonicalize root directory {}: {}",
                r.display(),
                e
            )
        })?
    } else {
        let current_dir = std::env::current_dir().map_err(|e| {
            anyhow::anyhow!("Failed to get current working directory: {e}. Hint: Your current directory might have been deleted.")
        })?;
        current_dir.canonicalize().map_err(|e| {
            anyhow::anyhow!(
                "Failed to canonicalize current directory {}: {}",
                current_dir.display(),
                e
            )
        })?
    };

    // 1. Resolve to an absolute path.
    // Use the provided root (canonicalized) to resolve relative paths.
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        canonical_root.join(path)
    };

    // 2. Find the longest existing ancestor.
    // We walk up until we find a path that exists.
    let mut ancestor = absolute_path.as_path();
    while !ancestor.exists() {
        match ancestor.parent() {
            Some(p) => ancestor = p,
            None => break, // Reached root, which should exist, but handle just in case
        }
    }

    // 3. Canonicalize the ancestor to resolve all symlinks/indirections.
    let canonical_ancestor = ancestor.canonicalize().map_err(|e| {
        anyhow::anyhow!(
            "Failed to canonicalize ancestor path {}: {}",
            ancestor.display(),
            e
        )
    })?;

    // 4. Verification: check if the resolved ancestor is within the allowed root.
    if !canonical_ancestor.starts_with(&canonical_root) {
        // Clean up Windows extended path prefix for display
        let clean_path = canonical_ancestor
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_owned();
        let clean_root = canonical_root
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_owned();

        anyhow::bail!(
            "Output path '{clean_path}' is outside the current working directory '{clean_root}'.\n\
             Hint: Use a relative path like './report.json' or run the command from the target directory."
        );
    }

    // 5. Check the "remainder" (the part that doesn't exist yet) for ".." components.
    // We can't rely on `canonicalize` for non-existent files.
    // We iterate over components of the original absolute path.
    // But since we may have resolved symlinks in the ancestor, comparing strings is tricky.
    // A simpler strict approach for the "rest" is: if the user provided components
    // for the non-existent part, they must be normal components.

    // We can strip the suffix (non-existent part) from the *original* absolute path.
    // However, `ancestor` was derived from `absolute_path` by stripping tail.
    // So the remainder is `absolute_path` stripped of `ancestor`.
    if let Ok(remainder) = absolute_path.strip_prefix(ancestor) {
        for component in remainder.components() {
            if let std::path::Component::ParentDir = component {
                anyhow::bail!(
                    "Security Error: Path contains '..' in non-existent portion: '{}'",
                    path.display()
                );
            }
        }
    }

    // Reconstruct the final path using the canonical ancestor + remainder to be safe and clean.
    // But we must be careful: if we return a path that looks different than what user gave,
    // they might be confused. However, returning the canonicalized version + clean remainder
    // is usually the most correct "safe" path.
    //
    // Let's rely on returning the original absolute path, now that we've verified it's safe.
    Ok(absolute_path)
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod tests;
