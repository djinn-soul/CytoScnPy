//! Parallel Python file scanner for function metric extraction.

use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;

use super::extractor::extract_functions_from_ast;
use super::types::{FunctionInfo, FunctionStats, FunctionsResult};
use crate::commands::utils::find_python_files;
use crate::utils::LineIndex;

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

    let scanned: Vec<Result<Vec<FunctionInfo>, String>> = python_files
        .par_iter()
        .map(|path| {
            let content = fs::read_to_string(path)
                .map_err(|error| format!("{}: read error: {error}", path.display()))?;
            let parsed = ruff_python_parser::parse_module(&content)
                .map_err(|error| format!("{}: Python parse error: {error}", path.display()))?;
            let index = LineIndex::new(&content);
            Ok(extract_functions_from_ast(
                &parsed.into_syntax(),
                &index,
                path,
            ))
        })
        .collect();
    let mut functions = Vec::new();
    let mut scan_issues = Vec::new();
    let mut files_scanned = 0;
    for result in scanned {
        match result {
            Ok(found) => {
                files_scanned += 1;
                functions.extend(found);
            }
            Err(issue) => scan_issues.push(issue),
        }
    }

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
        files_scanned,
        scan_issues,
    }
}

/// Analyzes Python functions across `roots` respecting `exclude` patterns.
#[must_use]
pub fn analyze_functions(roots: &[PathBuf], exclude: &[String], verbose: bool) -> FunctionsResult {
    let files = find_python_files(roots, exclude, verbose);
    scan_files(&files)
}
