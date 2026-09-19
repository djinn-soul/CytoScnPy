//! Pattern matching utilities for `--ignore` and exclusion handling across scanners.

use std::path::Path;

/// Checks whether a file or directory name matches an ignore pattern.
#[must_use]
pub fn is_name_or_pattern_ignored(name: &str, patterns: &[String]) -> bool {
    for pat in patterns {
        let clean = pat.trim_start_matches("./").trim_end_matches('/');
        if clean.is_empty() {
            continue;
        }
        if let Some(suffix) = clean.strip_prefix("*.") {
            if name.ends_with(suffix) {
                return true;
            }
        } else if let Some(prefix) = clean.strip_suffix("/*") {
            if name == prefix {
                return true;
            }
        } else if name == clean || name == pat {
            return true;
        }
    }
    false
}

/// Checks whether a given path matches any ignore or exclusion patterns.
///
/// Supports:
/// - Exact directory or file name (e.g. `legacy`, `dirty.py`)
/// - Extension wildcards (e.g. `*.gen.py`, `*.pyi`)
/// - Directory wildcard prefixes (e.g. `tests/*`, `vendor/*`)
/// - Slash-separated path subsegments (e.g. `sub/folder`)
#[must_use]
pub fn is_path_ignored(path: &Path, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }

    let path_str = path.to_string_lossy().replace('\\', "/");
    let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

    for pat in patterns {
        let clean = pat.trim_start_matches("./").trim_end_matches('/');
        if clean.is_empty() {
            continue;
        }
        if let Some(suffix) = clean.strip_prefix("*.") {
            if file_name.ends_with(suffix) || path_str.ends_with(suffix) {
                return true;
            }
        } else if let Some(prefix) = clean.strip_suffix("/*") {
            if path_str.contains(&format!("/{prefix}/"))
                || path_str.ends_with(&format!("/{prefix}"))
                || path_str.starts_with(&format!("{prefix}/"))
                || path_str == prefix
                || file_name == prefix
            {
                return true;
            }
        } else if clean.contains('/') {
            if path_str.contains(&format!("/{clean}/"))
                || path_str.ends_with(&format!("/{clean}"))
                || path_str.starts_with(&format!("{clean}/"))
                || path_str == clean
            {
                return true;
            }
        } else if file_name == clean || path_str.split('/').any(|comp| comp == clean) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_and_pattern_ignored() {
        let patterns = vec![
            "legacy/".to_owned(),
            "*.gen.py".to_owned(),
            "tests/*".to_owned(),
        ];
        assert!(is_name_or_pattern_ignored("legacy", &patterns));
        assert!(is_name_or_pattern_ignored("foo.gen.py", &patterns));
        assert!(is_name_or_pattern_ignored("tests", &patterns));
        assert!(!is_name_or_pattern_ignored("normal.py", &patterns));
    }

    #[test]
    fn test_path_ignored_various_patterns() {
        let patterns = vec![
            "temp.py".to_owned(),
            "*.tmp.py".to_owned(),
            "generated/*".to_owned(),
            "pkg/legacy".to_owned(),
        ];

        assert!(is_path_ignored(Path::new("src/temp.py"), &patterns));
        assert!(is_path_ignored(Path::new("src/app.tmp.py"), &patterns));
        assert!(is_path_ignored(Path::new("generated/module.py"), &patterns));
        assert!(is_path_ignored(
            Path::new("root/pkg/legacy/core.py"),
            &patterns
        ));
        assert!(!is_path_ignored(Path::new("src/main.py"), &patterns));
    }
}
