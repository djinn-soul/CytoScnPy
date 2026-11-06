//! Path utilities for CytoScnPy.
//!
//! This module consolidates all path-related logic for:
//! - Cross-platform path normalization
//! - Path traversal security validation
//! - Python file discovery with gitignore support

use crate::constants::{CONFIG_FILENAME, DEFAULT_EXCLUDE_FOLDERS, PYPROJECT_FILENAME};

mod discovery;
mod validation;

pub(crate) use discovery::collect_python_files_within;
pub use validation::{validate_output_path, validate_path_within_root, validate_source_path};

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

/// Formats `path:line` for display, naming the notebook cell when the line
/// counts from the start of one (`nb.ipynb[cell 3]:2`).
#[must_use]
pub fn display_location(path: &std::path::Path, cell: Option<usize>, line: usize) -> String {
    let path = normalize_display_path(path);
    match cell {
        Some(cell) => format!("{path}[cell {cell}]:{line}"),
        None => format!("{path}:{line}"),
    }
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
    collect_python_files_within(root, None, exclude, include, include_ipynb, verbose)
}

/// Like [`collect_python_files_gitignore_with_errors`], but symlinked sources
/// may resolve anywhere inside `boundary` instead of only inside `root`. Used
/// when `root` is nested inside another directory input of the same scan.
/// User excludes plus the default ones, minus force-included folders.
fn effective_excludes(exclude: &[String], include: &[String]) -> Vec<String> {
    let mut excludes: Vec<String> = exclude
        .iter()
        .cloned()
        .chain(DEFAULT_EXCLUDE_FOLDERS().iter().map(|&s| s.to_owned()))
        .collect();
    excludes.retain(|ex| !include.iter().any(|inc| ex == inc));
    excludes
}

/// Whether a directory walk skips the directory `name` at `rel` below its root.
fn is_dir_excluded(name: &str, rel: &std::path::Path, excludes: &[String]) -> bool {
    is_excluded(name, excludes) || super::is_path_ignored(rel, excludes)
}

/// Whether `file`, named directly as an input, lies in a directory that a
/// walk from `root` would skip. Only directories below `root` count, so an
/// unrelated ancestor such as `build/` cannot exclude a whole project.
pub(crate) fn is_file_in_excluded_dir(
    file: &std::path::Path,
    root: &std::path::Path,
    exclude: &[String],
    include: &[String],
) -> bool {
    // `Path::parent` of a bare file name is the empty path, which means the
    // working directory. Left empty it fails to canonicalize and strips from
    // any path, so every directory above the project would be checked.
    let root = if root.as_os_str().is_empty() {
        std::path::Path::new(".")
    } else {
        root
    };
    let file = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let Ok(rel) = file.strip_prefix(&root) else {
        return false;
    };
    let excludes = effective_excludes(exclude, include);
    let mut dir = std::path::PathBuf::new();
    for component in rel
        .parent()
        .into_iter()
        .flat_map(std::path::Path::components)
    {
        let std::path::Component::Normal(name) = component else {
            continue;
        };
        dir.push(name);
        if is_dir_excluded(&name.to_string_lossy(), &dir, &excludes) {
            return true;
        }
    }
    false
}

/// Walks `root` the way source collection does: respecting `.gitignore` and
/// skipping excluded directories at traversal time, so the walk never descends
/// into `node_modules`, `.venv`, `target` and the like.
fn source_walker(root: &std::path::Path, excludes_for_filter: Vec<String>) -> ignore::Walk {
    use ignore::WalkBuilder;

    let root_for_filter = root.to_path_buf();
    WalkBuilder::new(root)
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
                if is_dir_excluded(name, rel_entry, &excludes_for_filter) {
                    return false;
                }
            }

            true
        })
        .build()
}

/// Folders below `root`, and `root` itself, that hold their own CytoScnPy
/// configuration: a `.cytoscnpy.toml`, or a `pyproject.toml` with a
/// `[tool.cytoscnpy]` table. Folders a scan of `root` skips are not searched.
#[must_use]
pub fn find_project_roots(
    root: &std::path::Path,
    exclude: &[String],
    include: &[String],
) -> Vec<std::path::PathBuf> {
    let mut roots = vec![root.to_path_buf()];
    for entry in source_walker(root, effective_excludes(exclude, include)).flatten() {
        let path = entry.path();
        let configures = match path.file_name().and_then(|name| name.to_str()) {
            Some(crate::constants::CONFIG_FILENAME) => true,
            Some(crate::constants::PYPROJECT_FILENAME) => {
                crate::config::Config::configures_cytoscnpy(path)
            }
            _ => false,
        };
        if let Some(dir) = path.parent().filter(|_| configures) {
            if !roots.iter().any(|known| known == dir) {
                roots.push(dir.to_path_buf());
            }
        }
    }
    roots.sort();
    roots
}

/// Serializes unit tests that read or change the process working directory,
/// which is shared across the threads the test harness runs them on.
#[cfg(test)]
pub(crate) static CWD_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
#[path = "paths_tests.rs"]
mod tests;
