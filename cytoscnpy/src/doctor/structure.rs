use ignore::WalkBuilder;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::types::{LanguageStats, RepoStructureStats};

const SKIP_DIR_NAMES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
    ".pytest_cache",
    ".ruff_cache",
    ".mypy_cache",
    ".idea",
    ".vscode",
];

fn detect_language(ext: &str) -> Option<&'static str> {
    match ext {
        "py" | "pyi" => Some("Python"),
        "rs" => Some("Rust"),
        "js" | "jsx" | "mjs" | "cjs" => Some("JavaScript"),
        "ts" | "tsx" => Some("TypeScript"),
        "toml" => Some("TOML"),
        "json" => Some("JSON"),
        "yaml" | "yml" => Some("YAML"),
        "md" | "markdown" => Some("Markdown"),
        "sh" | "bash" => Some("Shell"),
        "go" => Some("Go"),
        "java" => Some("Java"),
        "c" | "h" => Some("C"),
        "cpp" | "hpp" | "cc" => Some("C++"),
        "html" | "htm" => Some("HTML"),
        "css" => Some("CSS"),
        _ => None,
    }
}

fn is_test_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();
    if path_str.contains("/tests/")
        || path_str.contains("/test/")
        || path_str.contains("\\tests\\")
        || path_str.contains("\\test\\")
    {
        return true;
    }

    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        let lower = file_name.to_lowercase();
        return lower.starts_with("test_")
            || lower.ends_with("_test.py")
            || lower.ends_with(".test.js")
            || lower.ends_with(".test.ts")
            || lower.ends_with(".spec.js")
            || lower.ends_with(".spec.ts")
            || lower == "conftest.py";
    }

    false
}

/// Scans the repository structure, calculating polyglot language breakdown and test ratio.
pub fn scan_repo_structure(repo_root: &Path) -> RepoStructureStats {
    let walker = WalkBuilder::new(repo_root)
        .standard_filters(true)
        .hidden(false)
        .filter_entry(|entry| {
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                if let Some(name) = entry.file_name().to_str() {
                    if SKIP_DIR_NAMES.contains(&name) {
                        return false;
                    }
                }
            }
            true
        })
        .build();

    let mut total_files = 0usize;
    let mut total_lines = 0usize;
    let mut total_bytes = 0u64;
    let mut source_files = 0usize;
    let mut source_lines = 0usize;
    let mut test_files = 0usize;
    let mut test_lines = 0usize;
    let mut max_depth = 0usize;

    let mut largest_file_path = String::new();
    let mut largest_file_lines = 0usize;

    let mut lang_map: HashMap<&'static str, (usize, usize, u64)> = HashMap::new();

    for result in walker {
        let Ok(entry) = result else {
            continue;
        };

        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        let Some(language) = detect_language(&ext) else {
            continue;
        };

        let Ok(meta) = fs::metadata(path) else {
            continue;
        };

        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };

        let lines = content.lines().count();
        let bytes = meta.len();

        total_files += 1;
        total_lines += lines;
        total_bytes += bytes;

        if lines > largest_file_lines {
            largest_file_lines = lines;
            largest_file_path = path
                .strip_prefix(repo_root)
                .unwrap_or(path)
                .display()
                .to_string();
        }

        if let Ok(rel) = path.strip_prefix(repo_root) {
            let depth = rel.components().count();
            if depth > max_depth {
                max_depth = depth;
            }
        }

        if is_test_file(path) {
            test_files += 1;
            test_lines += lines;
        } else {
            source_files += 1;
            source_lines += lines;
        }

        let entry = lang_map.entry(language).or_insert((0, 0, 0));
        entry.0 += 1;
        entry.1 += lines;
        entry.2 += bytes;
    }

    let mut languages: Vec<LanguageStats> = lang_map
        .into_iter()
        .map(|(lang, (files, lines, bytes))| LanguageStats {
            language: lang.to_owned(),
            file_count: files,
            line_count: lines,
            byte_count: bytes,
        })
        .collect();

    languages.sort_by_key(|a| std::cmp::Reverse(a.line_count));

    let test_to_source_ratio = if source_lines > 0 {
        test_lines as f64 / source_lines as f64
    } else {
        0.0
    };

    let avg_file_lines = total_lines.checked_div(total_files).unwrap_or(0);

    RepoStructureStats {
        total_files,
        total_lines,
        total_bytes,
        source_files,
        source_lines,
        test_files,
        test_lines,
        test_to_source_ratio,
        avg_file_lines,
        largest_file_path,
        largest_file_lines,
        max_directory_depth: max_depth,
        languages,
    }
}
