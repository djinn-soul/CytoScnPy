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
/// This ensures that the path acts within the Current Working Directory (CWD).
/// It resolves the longest existing ancestor to handle symlinks and checks
/// that the remaining path components do not contain `..` (`ParentDir`).
///
/// # Errors
///
/// Returns an error if:
/// - The current directory cannot be determined or resolved.
/// - The path traverses outside the current working directory.
/// - The path contains `..` components in the non-existent portion.
pub fn validate_output_path(path: &std::path::Path) -> anyhow::Result<std::path::PathBuf> {
    let current_dir = std::env::current_dir()?;
    let canonical_root = current_dir.canonicalize().map_err(|e| {
        anyhow::anyhow!(
            "Failed to canonicalize current directory {}: {}",
            current_dir.display(),
            e
        )
    })?;

    // 1. Resolve to an absolute path first.
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        current_dir.join(path)
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
        anyhow::bail!(
            "Security Error: Path traversal detected. Resolved path '{}' is outside the project root '{}'",
            canonical_ancestor.display(),
            canonical_root.display()
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
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn test_validate_path_within_root() -> anyhow::Result<()> {
        let parent = tempdir()?;
        let root = parent.path().join("root");
        fs::create_dir(&root)?;

        let inside = root.join("inside.txt");
        fs::write(&inside, "inside")?;

        let outside = parent.path().join("outside.txt");
        fs::write(&outside, "outside")?;

        // Valid path
        assert!(validate_path_within_root(&inside, &root).is_ok());

        // Path outside root (exists)
        assert!(validate_path_within_root(&outside, &root).is_err());

        // Traversal path (e.g. root/../outside.txt)
        let traversal = root.join("..").join("outside.txt");
        assert!(validate_path_within_root(&traversal, &root).is_err());

        Ok(())
    }

    // Helper to run test in a specific directory
    fn run_in_dir<F>(dir: &Path, f: F) -> anyhow::Result<()>
    where
        F: FnOnce() -> anyhow::Result<()>,
    {
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(dir)?;
        let result = f();
        std::env::set_current_dir(original_dir)?;
        result
    }

    #[test]
    fn test_validate_output_path_security() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        let root = temp_dir.path().join("project");
        fs::create_dir(&root)?;

        // Create a secret file outside
        let secret = temp_dir.path().join("secret.txt");
        fs::write(&secret, "super secret")?;

        run_in_dir(&root, || {
            // 1. Normal file in root
            let p1 = Path::new("report.json");
            let res1 = validate_output_path(p1);
            assert!(res1.is_ok(), "Simple relative path should be ok");
            let path1 = res1.unwrap();
            assert!(path1.starts_with(&root));

            // 2. File in subdir (subdir doesn't exist yet)
            let p2 = Path::new("sub/data/stats.txt");
            let res2 = validate_output_path(p2);
            assert!(res2.is_ok(), "Path in non-existent subdir should be ok");

            // 3. Traversal to outside
            let p3 = Path::new("../secret.txt");
            let res3 = validate_output_path(p3);
            assert!(res3.is_err(), "Traversal ../ should be blocked");

            // 4. Absolute path to outside
            let p4 = secret.as_path();
            let res4 = validate_output_path(p4);
            assert!(
                res4.is_err(),
                "Absolute path outside root should be blocked"
            );

            // 5. Logical traversal in non-existent part
            // root/sub/../../secret.txt (where 'sub' doesn't exist)
            let p5 = Path::new("sub/../../secret.txt");
            let res5 = validate_output_path(p5);
            assert!(res5.is_err(), "Logical ... traversal should be blocked");

            Ok(())
        })
    }
}
