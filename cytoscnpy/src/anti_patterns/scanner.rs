//! Parallel scanner for anti-patterns across Python source files.

use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::python::detect_anti_patterns;
use super::types::{AntiPatternMatch, AntiPatternStats, AntiPatternsResult};
use crate::commands::utils::find_python_files;

fn is_test_file(path: &Path) -> bool {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if stem.ends_with("_spec") {
        return true;
    }
    crate::utils::is_test_path(&path.to_string_lossy())
}

/// Aggregates detected anti-pattern matches into an [`AntiPatternsResult`].
fn aggregate(
    mut matches: Vec<AntiPatternMatch>,
    files_scanned: usize,
    roots: Vec<PathBuf>,
) -> AntiPatternsResult {
    matches.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));

    let mut stats = AntiPatternStats::default();
    let mut affected_files = HashSet::new();

    for m in &matches {
        stats.record(m.kind);
        affected_files.insert(&m.file);
    }
    stats.affected_files = affected_files.len();

    AntiPatternsResult {
        matches,
        stats,
        files_scanned,
        roots,
    }
}

/// Scans Python source files in parallel for anti-patterns.
#[must_use]
pub fn scan_files(files: &[PathBuf]) -> AntiPatternsResult {
    let python_files: Vec<&PathBuf> = files
        .iter()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e == "py" || e == "pyi")
                && !is_test_file(p)
                && !crate::utils::is_likely_minified(p, None)
        })
        .collect();

    let matches: Vec<AntiPatternMatch> = python_files
        .par_iter()
        .filter_map(|path| {
            let content = fs::read_to_string(path).ok()?;
            Some(detect_anti_patterns(&content, path))
        })
        .flatten()
        .collect();

    aggregate(matches, python_files.len(), Vec::new())
}

/// Scans roots for Python files and detects anti-patterns.
#[must_use]
pub fn scan_anti_patterns(
    roots: &[PathBuf],
    exclude: &[String],
    verbose: bool,
) -> AntiPatternsResult {
    let files = find_python_files(roots, exclude, verbose);
    let mut result = scan_files(&files);
    result.roots = roots.to_vec();
    result
}
