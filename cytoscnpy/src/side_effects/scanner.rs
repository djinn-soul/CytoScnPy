//! Parallel file scanner and discovery for module-level side-effects analysis.

use ignore::WalkBuilder;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::polyglot::detect_js_side_effects;
use super::python::detect_python_side_effects;
use super::types::{SideEffectMatch, SideEffectStats, SideEffectsResult};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceFileType {
    Python,
    JavaScript,
}

fn classify_file_type(path: &Path) -> Option<SourceFileType> {
    let ext = path.extension().and_then(|e| e.to_str())?;
    match ext {
        "py" | "pyi" => Some(SourceFileType::Python),
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
            if !entry.file_type().is_some_and(|ft| ft.is_file()) || is_test_file(path) {
                continue;
            }

            let path_str = path.to_string_lossy();
            if excludes.iter().any(|ex| path_str.contains(ex)) {
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

/// Scans source files in parallel for module-level side effects.
#[must_use]
pub fn scan_files(files: &[PathBuf]) -> SideEffectsResult {
    let mut matches: Vec<SideEffectMatch> = files
        .par_iter()
        .filter_map(|path| {
            let file_type = classify_file_type(path)?;
            let content = fs::read_to_string(path).ok()?;
            let file_matches = match file_type {
                SourceFileType::Python => detect_python_side_effects(&content, path),
                SourceFileType::JavaScript => detect_js_side_effects(&content, path),
            };
            Some(file_matches)
        })
        .flatten()
        .collect();

    matches.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));

    let mut stats = SideEffectStats::default();
    let mut affected_files = HashSet::new();

    for m in &matches {
        stats.record(m.kind);
        affected_files.insert(&m.file);
    }
    stats.affected_files = affected_files.len();

    SideEffectsResult {
        stats,
        files_scanned: files.len(),
        matches,
    }
}
