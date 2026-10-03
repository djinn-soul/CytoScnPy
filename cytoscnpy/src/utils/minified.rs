//! Heuristics for detecting minified source files.
//!
//! Minified and bundled files (such as `.min.py` bundles or single-line packed
//! payloads) distort AST complexity, cyclomatic metrics, nesting depth, and
//! duplicate detection. This module identifies likely minified files so that
//! AST-oriented analyses can exclude them safely.

use std::fs;
use std::path::Path;

/// Maximum average characters per line before a file is considered minified.
const MAX_AVG_LINE_LENGTH: f64 = 200.0;

/// Byte threshold for few-line bundled files (<= 3 lines).
const MINIFIED_FEW_LINES_BYTE_LIMIT: usize = 1024;

/// Maximum length for any single line before being flagged as a minified line.
const MAX_SINGLE_LINE_LENGTH: usize = 1500;

/// Minimum file size in bytes to bother checking line length heuristics.
const MIN_SIZE_FOR_HEURISTIC: u64 = 200;

/// Checks if a file path indicates a minified file based on its name.
///
/// Matches filenames containing `.min.` (e.g. `bundle.min.py`, `script.min.js`),
/// or ending in `-min.py` / `_min.py`.
#[must_use]
pub fn is_likely_minified_path(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let lower = file_name.to_lowercase();
    lower.contains(".min.")
        || lower.ends_with(".min.py")
        || lower.ends_with("-min.py")
        || lower.ends_with("_min.py")
}

/// Checks whether source text appears to be minified code.
///
/// Follows `DeSlopify`'s heuristic criteria:
/// 1. Very few lines (`<= 3`) with total size exceeding 1 KB (`> 1024` bytes).
/// 2. Average line length exceeding 200 characters.
/// 3. Any single line exceeding 1500 characters.
#[must_use]
pub fn is_likely_minified_content(content: &str) -> bool {
    if content.is_empty() {
        return false;
    }

    let mut line_count = 0usize;
    let mut max_line_len = 0usize;

    for line in content.lines() {
        line_count += 1;
        if line.len() > max_line_len {
            max_line_len = line.len();
        }
    }

    if line_count == 0 {
        return false;
    }

    let size_bytes = content.len();
    let avg_line_len = size_bytes as f64 / line_count as f64;

    (line_count <= 3 && size_bytes > MINIFIED_FEW_LINES_BYTE_LIMIT)
        || avg_line_len > MAX_AVG_LINE_LENGTH
        || max_line_len > MAX_SINGLE_LINE_LENGTH
}

/// Checks whether a file is likely minified based on path naming and content inspection.
///
/// If `content` is `None` and the file exists on disk, reads the file if its size exceeds
/// minimum threshold (`MIN_SIZE_FOR_HEURISTIC`).
#[must_use]
pub fn is_likely_minified(path: &Path, content: Option<&str>) -> bool {
    if is_likely_minified_path(path) {
        return true;
    }

    if let Some(text) = content {
        return is_likely_minified_content(text);
    }

    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };

    if !metadata.is_file() || metadata.len() < MIN_SIZE_FOR_HEURISTIC {
        return false;
    }

    // Inspect content from disk for files large enough to trigger minification heuristics
    match fs::read_to_string(path) {
        Ok(text) => is_likely_minified_content(&text),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_is_likely_minified_path() {
        assert!(is_likely_minified_path(Path::new("bundle.min.py")));
        assert!(is_likely_minified_path(Path::new("app-min.py")));
        assert!(is_likely_minified_path(Path::new("utils_min.py")));
        assert!(is_likely_minified_path(Path::new("vendor/jquery.min.js")));
        assert!(!is_likely_minified_path(Path::new("admin.py")));
        assert!(!is_likely_minified_path(Path::new("mini_app.py")));
        assert!(!is_likely_minified_path(Path::new("terminal.py")));
    }

    #[test]
    fn test_is_likely_minified_content_normal_code() {
        let code = "def hello():\n    print('world')\n\ndef add(a, b):\n    return a + b\n";
        assert!(!is_likely_minified_content(code));
        assert!(!is_likely_minified_content(""));
    }

    #[test]
    fn test_is_likely_minified_content_few_lines_large_size() {
        let big_line = "x = 1; ".repeat(200); // > 1400 chars
        assert!(is_likely_minified_content(&big_line));

        let two_lines = format!("{}\n{}", "a = 1; ".repeat(100), "b = 2; ".repeat(100));
        assert!(is_likely_minified_content(&two_lines));
    }

    #[test]
    fn test_is_likely_minified_content_high_average_length() {
        let lines = (0..5)
            .map(|_| "v = [i for i in range(100) if i % 2 == 0]; ".repeat(6))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(is_likely_minified_content(&lines));
    }

    #[test]
    fn test_is_likely_minified_disk_file() {
        use std::io::Write;

        let mut normal_file = NamedTempFile::new().unwrap();
        writeln!(normal_file, "def foo(): pass").unwrap();
        assert!(!is_likely_minified(normal_file.path(), None));

        let mut min_content_file = NamedTempFile::new().unwrap();
        let payload = "a = 1; ".repeat(250);
        write!(min_content_file, "{payload}").unwrap();
        assert!(is_likely_minified(min_content_file.path(), None));
    }
}
