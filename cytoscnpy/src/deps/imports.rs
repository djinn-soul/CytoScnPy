use crate::analyzer::types::ParseError;
use crate::commands::utils::find_python_files_with_issues;
use rayon::prelude::*;
use rustc_hash::FxHashSet;
use std::path::PathBuf;

#[path = "imports_collect.rs"]
mod imports_collect;
#[path = "imports_dynamic.rs"]
mod imports_dynamic;
#[path = "imports_source.rs"]
mod imports_source;
#[path = "imports_type_checking.rs"]
mod imports_type_checking;
use imports_source::extract_imports_from_file;

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
    /// Imports that appear only inside `if TYPE_CHECKING:` blocks. These are not
    /// runtime imports, so they must not create "missing dependency" findings, but
    /// they do keep a declared dependency from being reported as unused.
    pub type_checking: FxHashSet<String>,
    /// Source evidence for every import occurrence.
    pub occurrences: Vec<ImportOccurrence>,
    /// Files or directories that could not be read or parsed.
    pub scan_errors: Vec<ParseError>,
}

impl ImportScan {
    fn empty() -> Self {
        Self {
            all: FxHashSet::default(),
            production: FxHashSet::default(),
            type_checking: FxHashSet::default(),
            occurrences: Vec::new(),
            scan_errors: Vec::new(),
        }
    }
}

/// A top-level `setup.py` is a build script, not importable source: what it
/// imports (`setuptools`, `distutils`) are build requirements rather than
/// dependencies of the project, so it must not drive dependency findings.
fn is_build_script(file: &std::path::Path, roots: &[PathBuf]) -> bool {
    file.file_name().is_some_and(|name| name == "setup.py")
        && file
            .parent()
            .is_some_and(|parent| roots.iter().any(|root| same_dir(root, parent)))
}

/// Compares two directory paths without touching the filesystem. An empty path
/// and `.` both mean the current directory but are not equal as `Path` values.
fn same_dir(a: &std::path::Path, b: &std::path::Path) -> bool {
    fn as_current_dir(path: &std::path::Path) -> &std::path::Path {
        if path.as_os_str().is_empty() {
            std::path::Path::new(".")
        } else {
            path
        }
    }
    as_current_dir(a) == as_current_dir(b)
}

/// Scans Python files and returns import names split by all files and
/// production files. Test/dev files are excluded only from the production set.
pub fn extract_import_scan(roots: &[PathBuf], exclude: &[String], verbose: bool) -> ImportScan {
    let (mut files, issues) = find_python_files_with_issues(roots, exclude, verbose);
    let classification_roots: Vec<_> = roots
        .iter()
        .map(|root| {
            let directory = if root.is_file() {
                root.parent().unwrap_or(std::path::Path::new("."))
            } else {
                root.as_path()
            };
            super::declared::find_project_root(directory)
        })
        .collect();
    files.retain(|file| !is_build_script(file, roots));

    let mut scan = files
        .into_par_iter()
        .map(|file| {
            let root = classification_roots
                .iter()
                .find(|root| file.starts_with(root));
            let is_production = root.map_or_else(
                || {
                    !crate::utils::is_test_path(
                        &file.file_name().unwrap_or_default().to_string_lossy(),
                    )
                },
                |root| !crate::utils::is_test_path_relative_to(&file, root),
            );
            extract_imports_from_file(&file, is_production)
        })
        .reduce(ImportScan::empty, |mut acc, scan| {
            acc.all.extend(scan.all);
            acc.production.extend(scan.production);
            acc.type_checking.extend(scan.type_checking);
            acc.occurrences.extend(scan.occurrences);
            acc.scan_errors.extend(scan.scan_errors);
            acc
        });
    scan.scan_errors.extend(
        issues
            .into_iter()
            .map(|(file, error)| ParseError { file, error }),
    );
    scan.scan_errors
        .sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.error.cmp(&b.error)));
    scan
}

/// Scans Python files within the provided roots and extracts all import names,
/// including imports nested inside functions, classes, and control flow blocks.
pub fn extract_imports(roots: &[PathBuf], exclude: &[String], verbose: bool) -> FxHashSet<String> {
    let scan = extract_import_scan(roots, exclude, verbose);
    // This legacy set-only API cannot return diagnostics; keep failures visible.
    for error in &scan.scan_errors {
        eprintln!(
            "WARNING: Incomplete dependency import scan at {}: {}",
            error.file.display(),
            error.error
        );
    }
    scan.all
}

#[cfg(test)]
#[path = "imports_regression_tests.rs"]
mod regression_tests;
#[cfg(test)]
#[path = "imports_tests.rs"]
mod tests;
