//! Path utilities for CytoScnPy.
//!
//! This module consolidates all path-related logic for:
//! - Cross-platform path normalization
//! - Path traversal security validation
//! - Python file discovery with gitignore support

use crate::constants::DEFAULT_EXCLUDE_FOLDERS;

/// Normalizes a path for CLI display.
///
/// - Converts backslashes to forward slashes (for cross-platform consistency)
/// - Strips leading "./" or ".\" prefix (for cleaner output)
///
/// # Examples
/// ```
/// use std::path::Path;
/// use cytoscnpy::utils::normalize_display_path;
///
/// assert_eq!(normalize_display_path(Path::new(".\\benchmark\\test.py")), "benchmark/test.py");
/// assert_eq!(normalize_display_path(Path::new("./src/main.py")), "src/main.py");
/// ```
#[must_use]
pub fn normalize_display_path(path: &std::path::Path) -> String {
    let s = path.to_string_lossy();
    // Strip Windows extended path prefix if present
    let clean = s.trim_start_matches(r"\\?\");
    let normalized = clean.replace('\\', "/");
    normalized
        .strip_prefix("./")
        .unwrap_or(&normalized)
        .to_owned()
}

/// Checks if a name matches any exclusion pattern.
/// Supports exact matching and wildcard patterns starting with `*.`.
#[must_use]
pub fn is_excluded(name: &str, excludes: &[String]) -> bool {
    for exclude in excludes {
        if exclude.starts_with("*.") {
            if name.ends_with(&exclude[1..]) {
                return true;
            }
        } else if name == exclude {
            return true;
        }
    }
    false
}

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
    let current_dir = std::env::current_dir()?;
    let root_dir = root.unwrap_or(&current_dir);
    let canonical_root = root_dir.canonicalize().map_err(|e| {
        anyhow::anyhow!(
            "Failed to canonicalize root directory {}: {}",
            root_dir.display(),
            e
        )
    })?;

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

/// Collects Python files from a directory with gitignore support.
///
/// Uses the `ignore` crate to respect .gitignore, .git/info/exclude, and global gitignore
/// IN ADDITION to the hardcoded default exclusions (venv, `node_modules`, target, etc.).
///
/// # Arguments
/// * `root` - Root directory to search
/// * `exclude` - Additional user-specified exclusion patterns
/// * `include` - Folders to force-include (overrides excludes)
/// * `include_ipynb` - Whether to include .ipynb files
/// * `verbose` - Whether to print walk errors to stderr
///
/// # Returns
/// Tuple of (Vector of `PathBuf` for all Python files found, directory count)
#[must_use]
pub fn collect_python_files_gitignore(
    root: &std::path::Path,
    exclude: &[String],
    include: &[String],
    include_ipynb: bool,
    verbose: bool,
) -> (Vec<std::path::PathBuf>, usize) {
    use ignore::WalkBuilder;

    // Merge user excludes with default excludes
    let default_excludes: Vec<String> = DEFAULT_EXCLUDE_FOLDERS()
        .iter()
        .map(|&s| s.to_owned())
        .collect();
    let mut all_excludes: Vec<String> = exclude.iter().cloned().chain(default_excludes).collect();

    // Remove force-included folders from exclusion list
    all_excludes.retain(|ex| !include.iter().any(|inc| ex == inc));

    // Clone excludes for use in filter closure
    let excludes_for_filter = all_excludes.clone();
    let root_for_filter = root.to_path_buf();

    // Use ignore crate's WalkBuilder for gitignore support
    // Add filter_entry to skip excluded directories at traversal time,
    // preventing descent into node_modules, .venv, target, etc.
    let walker = WalkBuilder::new(root)
        .hidden(false) // Don't skip hidden files (we handle that with defaults)
        .git_ignore(true) // Respect .gitignore files
        .git_global(true) // Respect global gitignore
        .git_exclude(true) // Respect .git/info/exclude
        .filter_entry(move |entry| {
            // Always allow the root directory
            if entry.path() == root_for_filter {
                return true;
            }

            // Only filter directories - allow all files through (we filter them later)
            if !entry.file_type().is_some_and(|ft| ft.is_dir()) {
                return true;
            }

            // Check if directory name matches any exclusion pattern
            if let Some(name) = entry.file_name().to_str() {
                if is_excluded(name, &excludes_for_filter) {
                    return false;
                }
            }

            true
        })
        .build();

    let mut files = Vec::new();
    let mut dir_count = 0;

    for result in walker {
        if let Ok(entry) = result {
            let path = entry.path();

            // Count directories (excluded dirs won't appear here due to filter_entry)
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                if path != root {
                    dir_count += 1;
                }
                continue;
            }

            // Check file extension
            let is_python = path.extension().is_some_and(|ext| ext == "py");
            let is_notebook = include_ipynb && path.extension().is_some_and(|ext| ext == "ipynb");

            if !is_python && !is_notebook {
                continue;
            }

            files.push(path.to_path_buf());
        } else if verbose {
            // Ignore walk errors silently unless verbose
            if let Err(e) = result {
                eprintln!("Walk error: {e}");
            }
        }
    }

    (files, dir_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn test_validate_path_within_root() -> anyhow::Result<()> {
        let temp = tempdir()?;
        let root = temp.path();

        // Create nested structure
        let inside = root.join("subdir/file.py");
        fs::create_dir_all(
            inside
                .parent()
                .ok_or_else(|| anyhow::anyhow!("No parent"))?,
        )?;
        fs::write(&inside, "# test")?;

        // Test valid path (inside root)
        assert!(validate_path_within_root(&inside, root).is_ok());

        // Test invalid path (outside root via ..)
        let outside = root.join("../outside.py");
        assert!(validate_path_within_root(&outside, root).is_err());

        // Test path traversal
        let traversal = root.join("subdir/../../etc/passwd");
        assert!(validate_path_within_root(&traversal, root).is_err());

        Ok(())
    }

    /// Helper to run test in a specific directory
    fn run_in_dir<F>(dir: &Path, f: F) -> anyhow::Result<()>
    where
        F: FnOnce() -> anyhow::Result<()>,
    {
        let original = std::env::current_dir()?;
        std::env::set_current_dir(dir)?;
        let result = f();
        std::env::set_current_dir(original)?;
        result
    }

    #[test]
    fn test_validate_output_path_security() -> anyhow::Result<()> {
        let temp = tempdir()?;
        let root = temp.path();
        fs::create_dir_all(root.join("subdir"))?;

        run_in_dir(root, || {
            // Test valid relative paths
            assert!(validate_output_path(Path::new("./report.json"), None).is_ok());
            assert!(validate_output_path(Path::new("subdir/output.json"), None).is_ok());

            // Test path traversal attempts
            assert!(validate_output_path(Path::new("../outside.json"), None).is_err());
            assert!(validate_output_path(Path::new("subdir/../../escape.json"), None).is_err());

            // Test with explicit root
            assert!(validate_output_path(Path::new("./report.json"), Some(root)).is_ok());
            assert!(validate_output_path(Path::new("../outside.json"), Some(root)).is_err());

            Ok(())
        })
    }

    #[test]
    fn test_collect_python_files_exclusion() -> anyhow::Result<()> {
        let temp = tempdir()?;
        let root = temp.path();

        // Create Python files
        fs::write(root.join("main.py"), "# main")?;
        fs::write(root.join("app.py"), "# app")?;

        // Create excluded directories with Python files
        fs::create_dir_all(root.join(".venv"))?;
        fs::write(root.join(".venv/lib.py"), "# venv lib")?;

        fs::create_dir_all(root.join("node_modules"))?;
        fs::write(root.join("node_modules/script.py"), "# node")?;

        fs::create_dir_all(root.join("__pycache__"))?;
        fs::write(root.join("__pycache__/cached.py"), "# cached")?;

        // Create valid subdirectory
        fs::create_dir_all(root.join("src"))?;
        fs::write(root.join("src/module.py"), "# module")?;

        let (files, _) = collect_python_files_gitignore(root, &[], &[], false, false);

        // Should find main.py, app.py, src/module.py
        // Should NOT find .venv/lib.py, node_modules/script.py, __pycache__/cached.py
        assert_eq!(files.len(), 3);

        let file_names: Vec<_> = files
            .iter()
            .filter_map(|p| p.file_name())
            .filter_map(|f| f.to_str())
            .collect();

        assert!(file_names.contains(&"main.py"));
        assert!(file_names.contains(&"app.py"));
        assert!(file_names.contains(&"module.py"));
        assert!(!file_names.contains(&"lib.py"));
        assert!(!file_names.contains(&"script.py"));
        assert!(!file_names.contains(&"cached.py"));

        Ok(())
    }

    #[test]
    fn test_collect_python_files_force_include() -> anyhow::Result<()> {
        let temp = tempdir()?;
        let root = temp.path();

        // Create a normally-excluded directory that we want to force-include
        fs::create_dir_all(root.join("tests"))?;
        fs::write(root.join("tests/test_main.py"), "# test")?;

        // Force-include "tests" (which might be in DEFAULT_EXCLUDE_FOLDERS)
        // Note: tests is not actually in defaults, but this tests the mechanism
        let (files, _) =
            collect_python_files_gitignore(root, &[], &["tests".to_owned()], false, false);

        assert_eq!(files.len(), 1);

        Ok(())
    }

    #[test]
    fn test_collect_python_files_no_substring_unexclude() -> anyhow::Result<()> {
        let temp = tempdir()?;
        let root = temp.path();

        // Create structure where "tests" is force-included but ".venv" has "tests" substring
        fs::create_dir_all(root.join("tests"))?;
        fs::write(root.join("tests/test_file.py"), "# test")?;

        fs::create_dir_all(root.join(".venv/site-packages/tests"))?;
        fs::write(
            root.join(".venv/site-packages/tests/internal.py"),
            "# internal",
        )?;

        // Force include "tests" but .venv should still be excluded
        let (files, _) =
            collect_python_files_gitignore(root, &[], &["tests".to_owned()], false, false);

        // Should find tests/test_file.py but NOT .venv/site-packages/tests/internal.py
        assert_eq!(files.len(), 1);

        let file_names: Vec<_> = files
            .iter()
            .filter_map(|p| p.file_name())
            .filter_map(|f| f.to_str())
            .collect();

        assert!(file_names.contains(&"test_file.py"));
        assert!(!file_names.contains(&"internal.py"));

        Ok(())
    }
}
