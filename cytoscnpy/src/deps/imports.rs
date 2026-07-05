use crate::commands::utils::find_python_files;
use rayon::prelude::*;
use rustc_hash::FxHashSet;
use std::path::PathBuf;

#[path = "imports_collect.rs"]
mod imports_collect;
#[path = "imports_dynamic.rs"]
mod imports_dynamic;
#[path = "imports_type_checking.rs"]
mod imports_type_checking;
use imports_collect::extract_imports_from_file;

/// Concrete source location for a top-level import name.
#[derive(Debug, Clone)]
pub struct ImportOccurrence {
    /// Top-level import name seen in source code.
    pub name: String,
    /// Python file containing the import.
    pub file: PathBuf,
    /// 1-indexed source line.
    pub line: usize,
    /// 1-indexed source column.
    pub column: usize,
    /// Whether the file is classified as production code.
    pub is_production: bool,
}

/// Import names split by all files and production files.
pub struct ImportScan {
    /// Imports found anywhere in scanned Python files.
    pub all: FxHashSet<String>,
    /// Imports found in files classified as production code.
    pub production: FxHashSet<String>,
    /// Source evidence for every import occurrence.
    pub occurrences: Vec<ImportOccurrence>,
}

fn is_test_or_dev_file(file: &std::path::Path) -> bool {
    crate::utils::is_test_path(&file.to_string_lossy())
}

/// Scans Python files and returns import names split by all files and
/// production files. Test/dev files are excluded only from the production set.
pub fn extract_import_scan(roots: &[PathBuf], exclude: &[String], verbose: bool) -> ImportScan {
    let files = find_python_files(roots, exclude, verbose);

    files
        .into_par_iter()
        .map(|file| {
            let is_production = !is_test_or_dev_file(&file);
            extract_imports_from_file(&file, is_production)
        })
        .reduce(
            || ImportScan {
                all: FxHashSet::default(),
                production: FxHashSet::default(),
                occurrences: Vec::new(),
            },
            |mut acc, scan| {
                acc.all.extend(scan.all);
                acc.production.extend(scan.production);
                acc.occurrences.extend(scan.occurrences);
                acc
            },
        )
}

/// Scans Python files within the provided roots and extracts all import names,
/// including imports nested inside functions, classes, and control flow blocks.
pub fn extract_imports(roots: &[PathBuf], exclude: &[String], verbose: bool) -> FxHashSet<String> {
    extract_import_scan(roots, exclude, verbose).all
}

#[cfg(test)]
#[path = "imports_tests.rs"]
mod tests;
