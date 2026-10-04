//! Scanner for TODO/FIXME/HACK/XXX annotations, debug prints, and commented-out code.

use rayon::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;

use crate::commands::utils::find_python_files;

use super::types::{ScanError, TodoMatch, TodoStats, TodosResult};

#[path = "content.rs"]
mod content;
pub use content::scan_file_content;
pub(crate) use content::skip_context_sensitive;

/// Scans a set of source files in parallel and returns an aggregated `TodosResult`.
pub fn scan_files(paths: &[PathBuf]) -> TodosResult {
    scan_files_with_sources(paths, None)
}

pub(crate) fn scan_files_with_sources(
    paths: &[PathBuf],
    sources: Option<&crate::utils::sources::SourceCache>,
) -> TodosResult {
    let (matches, errors): (Vec<Vec<TodoMatch>>, Vec<Option<ScanError>>) = paths
        .par_iter()
        .map(|path| {
            match sources.map_or_else(
                || std::fs::read_to_string(path).map_err(|error| error.to_string()),
                |sources| {
                    sources
                        .get(path)
                        .ok_or_else(|| "missing source inventory entry".to_owned())?
                        .as_ref()
                        .map(|source| source.content.clone())
                        .map_err(Clone::clone)
                },
            ) {
                Ok(content) => {
                    let mut matches = Vec::new();
                    scan_file_content(path, &content, skip_context_sensitive(path), &mut matches);
                    (matches, None)
                }
                Err(error) => (
                    Vec::new(),
                    Some(ScanError {
                        file: path.clone(),
                        error,
                    }),
                ),
            }
        })
        .unzip();
    let mut all_matches: Vec<_> = matches.into_iter().flatten().collect();
    let mut scan_errors: Vec<_> = errors.into_iter().flatten().collect();
    all_matches.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
    scan_errors.sort_by(|a, b| a.file.cmp(&b.file));

    let mut stats = TodoStats::default();
    let mut affected = HashSet::new();
    for m in &all_matches {
        stats.increment(m.kind);
        affected.insert(&m.file);
    }
    stats.affected_files = affected.len();
    TodosResult {
        stats,
        matches: all_matches,
        scan_errors,
    }
}

/// Top-level entry point: discovers source files under `roots`, applies
/// `exclude` patterns, then delegates to [`scan_files`].
pub fn scan_todos(roots: &[PathBuf], exclude: &[String], verbose: bool) -> TodosResult {
    let files = find_python_files(roots, exclude, verbose);
    scan_files(&files)
}
