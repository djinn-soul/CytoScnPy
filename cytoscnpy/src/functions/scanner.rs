//! Parallel Python file scanner for function metric extraction.

use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;

use super::extractor::extract_functions;
use super::types::{FunctionInfo, FunctionStats, FunctionsResult};
use crate::commands::utils::find_python_files;

/// Scans the given Python files in parallel and extracts all functions.
#[must_use]
pub fn scan_files(files: &[PathBuf]) -> FunctionsResult {
    let python_files: Vec<&PathBuf> = files
        .iter()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e == "py" || e == "pyi")
        })
        .collect();

    let mut functions: Vec<FunctionInfo> = python_files
        .par_iter()
        .filter_map(|path| {
            let content = fs::read_to_string(path).ok()?;
            Some(extract_functions(&content, path))
        })
        .flatten()
        .collect();

    functions.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.start_line.cmp(&b.start_line))
            .then(a.name.cmp(&b.name))
    });

    let stats = FunctionStats::from_functions(&functions);

    FunctionsResult {
        functions,
        stats,
        files_scanned: python_files.len(),
    }
}

/// Analyzes Python functions across `roots` respecting `exclude` patterns.
#[must_use]
pub fn analyze_functions(roots: &[PathBuf], exclude: &[String], verbose: bool) -> FunctionsResult {
    let files = find_python_files(roots, exclude, verbose);
    scan_files(&files)
}
