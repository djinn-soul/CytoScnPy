//! File scanning and discovery for mutable global state analysis.

use ignore::WalkBuilder;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::polyglot::{detect_js_globals, detect_rust_globals};
use super::python::detect_python_globals;
use super::types::{GlobalKind, GlobalMatch, GlobalsResult};

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

/// Checks whether a given path corresponds to a test context.
pub fn is_test_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();
    if path_str.contains("/tests/")
        || path_str.contains("/test/")
        || path_str.contains("\\tests\\")
        || path_str.contains("\\test\\")
        || path_str.contains("/spec/")
        || path_str.contains("\\spec\\")
    {
        return true;
    }

    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        let lower = file_name.to_lowercase();
        return lower.starts_with("test_")
            || lower.ends_with("_test.py")
            || lower.ends_with("_spec.py")
            || lower.ends_with(".test.js")
            || lower.ends_with(".test.ts")
            || lower.ends_with(".spec.js")
            || lower.ends_with(".spec.ts")
            || lower == "conftest.py";
    }

    false
}

/// Identifies the file type for analysis dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceFileType {
    Python,
    Rust,
    JavaScript,
}

fn classify_file_type(path: &Path) -> Option<SourceFileType> {
    let ext = path.extension().and_then(|e| e.to_str())?;
    match ext {
        "py" | "pyi" => Some(SourceFileType::Python),
        "rs" => Some(SourceFileType::Rust),
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" => Some(SourceFileType::JavaScript),
        _ => None,
    }
}

/// Discovers analyzable source files for the given targets and exclusions.
#[must_use]
pub fn collect_files(roots: &[PathBuf], excludes: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for root in roots {
        if root.is_file() {
            if classify_file_type(root).is_some() && !is_test_file(root) {
                files.push(root.clone());
            }
            continue;
        }

        let root_buf = root.clone();
        let mut builder = WalkBuilder::new(root);
        builder.hidden(true).git_ignore(true).git_global(true);
        builder.filter_entry(move |entry| {
            if entry.path() == root_buf {
                return true;
            }
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                if let Some(dir_name) = entry.file_name().to_str() {
                    if SKIP_DIR_NAMES.contains(&dir_name) {
                        return false;
                    }
                }
            }
            true
        });

        for entry in builder.build().flatten() {
            let path = entry.path();
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }

            if is_test_file(path) {
                continue;
            }

            if crate::utils::is_path_ignored(path, excludes) {
                continue;
            }

            if classify_file_type(path).is_some() {
                files.push(path.to_path_buf());
            }
        }
    }

    files.sort();
    files.dedup();
    files
}

/// Scans the given files in parallel for mutable global state.
#[must_use]
pub fn scan_files(files: &[PathBuf], target_root: &Path) -> GlobalsResult {
    let matches: Vec<GlobalMatch> = files
        .par_iter()
        .filter_map(|path| {
            let file_type = classify_file_type(path)?;
            let content = fs::read_to_string(path).ok()?;
            let file_matches = match file_type {
                SourceFileType::Python => detect_python_globals(&content, path),
                SourceFileType::Rust => detect_rust_globals(&content, path),
                SourceFileType::JavaScript => detect_js_globals(&content, path),
            };
            Some(file_matches)
        })
        .flatten()
        .collect();

    let mut result = GlobalsResult::new(target_root.to_path_buf());
    result.files_scanned = files.len();

    let mut affected_files = HashSet::new();
    for m in &matches {
        affected_files.insert(&m.file);
        match m.kind {
            GlobalKind::ModuleCollection => result.stats.module_collection_count += 1,
            GlobalKind::ClassVariable => result.stats.class_variable_count += 1,
            GlobalKind::GlobalMutation => result.stats.global_mutation_count += 1,
            GlobalKind::RustStaticMut => result.stats.rust_static_mut_count += 1,
            GlobalKind::JsTopLevelMutable => result.stats.js_toplevel_count += 1,
        }
    }

    result.stats.total_globals = matches.len();
    result.stats.affected_files = affected_files.len();
    result.matches = matches;
    result
        .matches
        .sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));

    result
}
