//! Path utilities for CytoScnPy.
//!
//! This module consolidates all path-related logic for:
//! - Cross-platform path normalization
//! - Path traversal security validation
//! - Python file discovery with gitignore support

use crate::constants::{CONFIG_FILENAME, DEFAULT_EXCLUDE_FOLDERS, PYPROJECT_FILENAME};

mod validation;
pub use validation::{validate_output_path, validate_path_within_root};

/// Finds a project boundary for file-only analysis without inspecting absolute
/// ancestors for test names. An unmarked input keeps its containing directory.
pub(crate) fn discover_project_root(directory: &std::path::Path) -> std::path::PathBuf {
    directory
        .ancestors()
        .find(|ancestor| {
            ancestor.join(CONFIG_FILENAME).is_file()
                || ancestor.join(PYPROJECT_FILENAME).is_file()
                || ancestor.join("setup.py").is_file()
                || ancestor.join("setup.cfg").is_file()
                || ancestor.join(".git").exists()
        })
        .unwrap_or(directory)
        .to_path_buf()
}

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
    super::is_name_or_pattern_ignored(name, excludes)
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
    let (files, directories, _) =
        collect_python_files_gitignore_with_errors(root, exclude, include, include_ipynb, verbose);
    (files, directories)
}

/// Discover Python files while returning traversal failures to callers that enforce scan integrity.
pub fn collect_python_files_gitignore_with_errors(
    root: &std::path::Path,
    exclude: &[String],
    include: &[String],
    include_ipynb: bool,
    verbose: bool,
) -> (Vec<std::path::PathBuf>, usize, Vec<String>) {
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

            // Check if directory matches any exclusion or ignore pattern
            if let Some(name) = entry.file_name().to_str() {
                let rel_entry = entry
                    .path()
                    .strip_prefix(&root_for_filter)
                    .unwrap_or(entry.path());
                if is_excluded(name, &excludes_for_filter)
                    || super::is_path_ignored(rel_entry, &excludes_for_filter)
                {
                    return false;
                }
            }

            true
        })
        .build();

    let mut files = Vec::new();
    let mut dir_count = 0;
    let mut errors = Vec::new();

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

            let rel_path = path.strip_prefix(root).unwrap_or(path);
            if super::is_path_ignored(rel_path, &all_excludes) {
                continue;
            }

            files.push(path.to_path_buf());
        } else if let Err(e) = result {
            if verbose {
                eprintln!("Walk error: {e}");
            }
            errors.push(e.to_string());
        }
    }

    (files, dir_count, errors)
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod tests;
