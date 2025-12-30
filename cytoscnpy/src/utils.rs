use crate::constants::{DEFAULT_EXCLUDE_FOLDERS, FRAMEWORK_FILE_RE, TEST_FILE_RE};
use ruff_text_size::TextSize;
use rustc_hash::FxHashSet;

/// A utility struct to convert byte offsets to line numbers.
///
/// This is necessary because the AST parser works with byte offsets,
/// but we want to report findings with line numbers which are more human-readable.
#[derive(Debug, Clone)]
pub struct LineIndex {
    /// Stores the byte index of the start of each line.
    line_starts: Vec<usize>,
}

impl LineIndex {
    /// Creates a new `LineIndex` by scanning the source code for newlines.
    /// Uses byte iteration for performance since '\n' is always a single byte in UTF-8.
    #[must_use]
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        // Use bytes() instead of char_indices() - newlines are always single bytes in UTF-8
        for (i, byte) in source.as_bytes().iter().enumerate() {
            if *byte == b'\n' {
                // Record the start of the next line (current newline index + 1)
                line_starts.push(i + 1);
            }
        }
        Self { line_starts }
    }

    /// Converts a `TextSize` (byte offset) to a 1-indexed line number.
    #[must_use]
    pub fn line_index(&self, offset: TextSize) -> usize {
        let offset = offset.to_usize();
        // Binary search to find which line range the offset falls into.
        match self.line_starts.binary_search(&offset) {
            Ok(line) => line + 1,
            Err(line) => line,
        }
    }
}

/// Detects lines with `# pragma: no cytoscnpy` comment.
///
/// Returns a set of line numbers (1-indexed) that should be ignored by the analyzer.
/// This allows users to suppress false positives or intentionally ignore specific lines.
#[must_use]
pub fn get_ignored_lines(source: &str) -> FxHashSet<usize> {
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains("pragma: no cytoscnpy"))
        .map(|(i, _)| i + 1)
        .collect()
}

/// Checks if a path is a test path.
#[must_use]
pub fn is_test_path(p: &str) -> bool {
    TEST_FILE_RE().is_match(p)
}

/// Checks if a path is a framework path.
#[must_use]
pub fn is_framework_path(p: &str) -> bool {
    FRAMEWORK_FILE_RE().is_match(p)
}

/// Parses exclude folders, combining defaults with user inputs.
pub fn parse_exclude_folders<S: std::hash::BuildHasher>(
    user_exclude_folders: Option<std::collections::HashSet<String, S>>,
    use_defaults: bool,
    include_folders: Option<std::collections::HashSet<String, S>>,
) -> FxHashSet<String> {
    let mut exclude_folders = FxHashSet::default();

    if use_defaults {
        for folder in DEFAULT_EXCLUDE_FOLDERS() {
            exclude_folders.insert((*folder).to_owned());
        }
    }

    if let Some(user_folders) = user_exclude_folders {
        exclude_folders.extend(user_folders);
    }

    if let Some(include) = include_folders {
        for folder in include {
            exclude_folders.remove(&folder);
        }
    }

    exclude_folders
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
    let normalized = s.replace('\\', "/");
    normalized
        .strip_prefix("./")
        .unwrap_or(&normalized)
        .to_owned()
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
///
/// # Returns
/// Tuple of (Vector of PathBuf for all Python files found, directory count)
#[must_use]
pub fn collect_python_files_gitignore(
    root: &std::path::Path,
    exclude: &[String],
    include: &[String],
    include_ipynb: bool,
) -> (Vec<std::path::PathBuf>, usize) {
    use ignore::WalkBuilder;

    // Merge user excludes with default excludes
    let default_excludes: Vec<String> = DEFAULT_EXCLUDE_FOLDERS()
        .iter()
        .map(|&s| s.to_owned())
        .collect();
    let mut all_excludes: Vec<String> = exclude.iter().cloned().chain(default_excludes).collect();

    // Remove force-included folders from exclusion list
    all_excludes.retain(|ex| {
        !include
            .iter()
            .any(|inc| ex.contains(inc) || inc.contains(ex))
    });

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
                for exclude in &excludes_for_filter {
                    if name.contains(exclude.as_str()) {
                        // Skip this directory entirely - don't descend into it
                        return false;
                    }
                }
            }

            true
        })
        .build();

    let mut files = Vec::new();
    let mut dir_count = 0;

    for result in walker {
        match result {
            Ok(entry) => {
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
                let is_notebook =
                    include_ipynb && path.extension().is_some_and(|ext| ext == "ipynb");

                if !is_python && !is_notebook {
                    continue;
                }

                files.push(path.to_path_buf());
            }
            Err(_) => {
                // Ignore walk errors silently
            }
        }
    }

    (files, dir_count)
}
