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

/// Detects lines with suppression comments.
///
/// Supports multiple formats:
/// - `# pragma: no cytoscnpy` - Legacy format
/// - `# noqa: CSP` - Standard Python linter format
///
/// Returns a set of line numbers (1-indexed) that should be ignored by the analyzer.
/// This allows users to suppress false positives or intentionally ignore specific lines.
#[must_use]
pub fn get_ignored_lines(source: &str) -> FxHashSet<usize> {
    let patterns = crate::constants::SUPPRESSION_PATTERNS();
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| patterns.iter().any(|pattern| line.contains(pattern)))
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

        let canonical_root = root.canonicalize()?;
        run_in_dir(&root, || {
            // 1. Normal file in root
            let p1 = Path::new("report.json");
            let res1 = validate_output_path(p1, None);
            assert!(res1.is_ok(), "Simple relative path should be ok");
            let path1 = res1.unwrap();
            assert!(path1.starts_with(&canonical_root));

            // 2. File in subdir (subdir doesn't exist yet)
            let p2 = Path::new("sub/data/stats.txt");
            let res2 = validate_output_path(p2, None);
            assert!(res2.is_ok(), "Path in non-existent subdir should be ok");

            // 3. Traversal to outside
            let p3 = Path::new("../secret.txt");
            let res3 = validate_output_path(p3, None);
            assert!(res3.is_err(), "Traversal ../ should be blocked");

            // 4. Absolute path to outside
            let p4 = secret.as_path();
            let res4 = validate_output_path(p4, None);
            assert!(
                res4.is_err(),
                "Absolute path outside root should be blocked"
            );

            // 5. Logical traversal in non-existent part
            // root/sub/../../secret.txt (where 'sub' doesn't exist)
            let p5 = Path::new("sub/../../secret.txt");
            let res5 = validate_output_path(p5, None);
            assert!(res5.is_err(), "Logical ... traversal should be blocked");

            Ok(())
        })
    }

    #[test]
    fn test_collect_python_files_exclusion() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        let root = temp_dir.path();

        // Create a legitimate folder that contains "git" (which is a default exclude prefix or similar)
        // Actually, default excludes are "venv", ".git", "node_modules", etc.
        // If we use "contains", then "widget" contains "git" if ".git" was matched by substring?
        // No, ".git" doesn't match "widget" via contains unless we did it the other way around.
        // BUT if the user excludes "git", then "widget" would be skipped.
        // Also "convenience" contains "venv" if "venv" was excluded.

        let widget_dir = root.join("widget");
        fs::create_dir(&widget_dir)?;
        fs::write(widget_dir.join("a.py"), "print('hello')")?;

        let convenience_dir = root.join("convenience");
        fs::create_dir(&convenience_dir)?;
        fs::write(convenience_dir.join("b.py"), "print('world')")?;

        let venv_dir = root.join("venv");
        fs::create_dir(&venv_dir)?;
        fs::write(venv_dir.join("c.py"), "print('venv')")?;

        let git_dir = root.join(".git");
        fs::create_dir(&git_dir)?;
        fs::write(git_dir.join("d.py"), "print('git')")?;

        // Test with "venv" and ".git" excluded (default behavior)
        let (files, _) = collect_python_files_gitignore(root, &[], &[], false, false);

        let file_names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        // "a.py" and "b.py" should be found. "c.py" and "d.py" should NOT.
        assert!(file_names.contains(&"a.py".to_owned()));
        assert!(file_names.contains(&"b.py".to_owned()));
        assert!(!file_names.contains(&"c.py".to_owned()));
        assert!(!file_names.contains(&"d.py".to_owned()));

        // Test with wildcard exclude "*.egg-info"
        let egg_info_dir = root.join("test.egg-info");
        fs::create_dir(&egg_info_dir)?;
        fs::write(egg_info_dir.join("e.py"), "print('egg-info')")?;

        let (files_wildcard, _) = collect_python_files_gitignore(root, &[], &[], false, false);
        let names_wildcard: Vec<String> = files_wildcard
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(
            !names_wildcard.contains(&"e.py".to_owned()),
            "Files in test.egg-info should be excluded by *.egg-info default exclude"
        );

        // Now test with a custom exclude "git"
        // In the old logic, this would exclude "widget"
        let (files2, _) =
            collect_python_files_gitignore(root, &["git".to_owned()], &[], false, false);
        let file_names2: Vec<String> = files2
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        // "a.py" (in "widget") should be found in new logic, but was missing in old logic
        assert!(
            file_names2.contains(&"a.py".to_owned()),
            "widget/a.py should be found even if 'git' is excluded"
        );

        Ok(())
    }

    #[test]
    fn test_collect_python_files_force_include() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        let root = temp_dir.path();

        let venv_dir = root.join("venv");
        fs::create_dir(&venv_dir)?;
        fs::write(venv_dir.join("a.py"), "print('venv')")?;

        // Test with "venv" excluded by default
        let (files, _) = collect_python_files_gitignore(root, &[], &[], false, false);
        assert!(files.is_empty(), "venv should be excluded by default");

        // Test with "venv" force-included
        #[allow(clippy::str_to_string)]
        let (files2, _) =
            collect_python_files_gitignore(root, &[], &["venv".to_string()], false, false);
        assert_eq!(
            files2.len(),
            1,
            "venv should be included if explicitly force-included"
        );
        assert_eq!(files2[0].file_name().unwrap().to_string_lossy(), "a.py");

        Ok(())
    }

    #[test]
    fn test_collect_python_files_no_substring_unexclude() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        let root = temp_dir.path();

        // Create .venv (excluded by default)
        let venv_dir = root.join(".venv");
        fs::create_dir(&venv_dir)?;
        fs::write(venv_dir.join("lib.py"), "print('library')")?;

        // Create .git (excluded by default)
        let git_dir = root.join(".git");
        fs::create_dir(&git_dir)?;
        fs::write(git_dir.join("config"), "git config")?;

        // 1. naive include="env" should NOT un-exclude ".venv"
        //    because "env" != ".venv"
        let (files_1, _) =
            collect_python_files_gitignore(root, &[], &["env".to_owned()], false, false);
        // files_1 should be empty because .venv remains excluded
        assert!(
            files_1.is_empty(),
            "include='env' should NOT un-exclude '.venv'"
        );

        // 2. include="git" should NOT un-exclude ".git"
        let (files_2, _) =
            collect_python_files_gitignore(root, &[], &["git".to_owned()], false, false);
        assert!(
            files_2.is_empty(),
            "include='git' should NOT un-exclude '.git'"
        );

        // 3. include=".venv" SHOULD un-exclude ".venv" (exact match)
        let (files_3, _) =
            collect_python_files_gitignore(root, &[], &[".venv".to_owned()], false, false);
        let names_3: Vec<String> = files_3
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(
            names_3.contains(&"lib.py".to_owned()),
            "Exact include='.venv' MUST un-exclude .venv"
        );

        Ok(())
    }
}
