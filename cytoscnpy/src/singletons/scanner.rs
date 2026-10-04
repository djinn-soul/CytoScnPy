//! Parallel Python file scanner for singleton pattern detection.

use rayon::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;

use super::python::detect_python_singletons_ast;
use super::types::{SingletonMatch, SingletonStats, SingletonsResult};

/// Aggregates detected singleton matches into a [`SingletonsResult`].
fn aggregate(mut matches: Vec<SingletonMatch>, files_scanned: usize) -> SingletonsResult {
    matches.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));

    let mut stats = SingletonStats::default();
    let mut affected_files = HashSet::new();

    for m in &matches {
        stats.record(m.kind);
        affected_files.insert(&m.file);
    }
    stats.affected_files = affected_files.len();

    SingletonsResult {
        stats,
        files_scanned,
        matches,
    }
}

/// Scans Python source files in parallel for singleton patterns.
#[must_use]
pub fn scan_files(files: &[PathBuf]) -> SingletonsResult {
    scan_files_with_sources(files, None)
}

pub(crate) fn scan_files_with_sources(
    files: &[PathBuf],
    sources: Option<&crate::utils::sources::SourceCache>,
) -> SingletonsResult {
    let python_files: Vec<&PathBuf> = files
        .iter()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e == "py" || e == "pyi")
        })
        .collect();

    let matches: Vec<SingletonMatch> = python_files
        .par_iter()
        .filter_map(|path| {
            let source = crate::utils::sources::load_source(path, sources).ok()?;
            Some(detect_python_singletons_ast(
                &source.content,
                path,
                &source.module.body,
            ))
        })
        .flatten()
        .collect();

    aggregate(matches, python_files.len())
}
