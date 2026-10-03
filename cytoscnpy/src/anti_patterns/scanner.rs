//! Parallel scanner for anti-patterns across Python source files.

use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::python::detect_anti_patterns;
use super::types::{AntiPatternMatch, AntiPatternStats, AntiPatternsResult, ScanError};
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
    scan_errors: Vec<ScanError>,
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
        scan_errors,
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

    let (matches, mut scan_errors): (Vec<AntiPatternMatch>, Vec<ScanError>) = python_files
        .par_iter()
        .map(|path| match fs::read_to_string(path) {
            Ok(content) => (detect_anti_patterns(&content, path), None),
            Err(error) => (
                Vec::new(),
                Some(ScanError {
                    file: (*path).clone(),
                    error: error.to_string(),
                }),
            ),
        })
        .fold(
            || (Vec::new(), Vec::new()),
            |(mut matches, mut errors), (file_matches, error)| {
                matches.extend(file_matches);
                errors.extend(error);
                (matches, errors)
            },
        )
        .reduce(
            || (Vec::new(), Vec::new()),
            |(mut matches_a, mut errors_a), (matches_b, errors_b)| {
                matches_a.extend(matches_b);
                errors_a.extend(errors_b);
                (matches_a, errors_a)
            },
        );

    scan_errors.sort_by(|a, b| a.file.cmp(&b.file));
    let files_scanned = python_files.len() - scan_errors.len();
    aggregate(matches, files_scanned, Vec::new(), scan_errors)
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
