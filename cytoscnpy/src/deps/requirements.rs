//! Requirements include traversal retains read errors and partial declarations.

use super::{
    extract_pep508_parts, normalize_package_name, DeclarationScan, DeclaredDependency,
    DependencySource,
};
use crate::analyzer::types::ParseError;
use std::path::{Path, PathBuf};

/// Legacy declaration-only parser. Read errors are printed as warnings.
/// Use `scan_requirements` when callers need to decide whether the scan is complete.
pub fn parse_requirements(path: &Path) -> Vec<DeclaredDependency> {
    let scan = scan_requirements(path);
    for error in &scan.scan_errors {
        eprintln!(
            "WARNING: Incomplete requirements scan at {}: {}",
            error.file.display(),
            error.error
        );
    }
    scan.dependencies
}

/// Parse requirements and their include files, retaining every source read failure.
pub fn scan_requirements(path: &Path) -> DeclarationScan {
    let mut scan = DeclarationScan::default();
    parse_requirements_inner(path, &mut Vec::new(), &mut scan);
    scan
}

fn requirement_include_target(line: &str) -> Option<&str> {
    let rest = line
        .strip_prefix("--requirement")
        .or_else(|| line.strip_prefix("-r"))?;
    let rest = rest.trim_start_matches(['=', ' ', '\t']).trim();
    (!rest.is_empty() && !rest.starts_with('-')).then_some(rest)
}

fn parse_requirements_inner(path: &Path, visited: &mut Vec<PathBuf>, scan: &mut DeclarationScan) {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    // Repeated includes are intentionally ignored to avoid cycles and duplicate declarations.
    if visited.contains(&canonical) {
        return;
    }
    visited.push(canonical);
    let filename = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let is_dev = filename.contains("dev") || filename.contains("test");
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            scan.scan_errors.push(ParseError {
                file: path.to_path_buf(),
                error: format!("Failed to read requirements file: {error}"),
            });
            return;
        }
    };
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('-') {
            if let Some(target) = requirement_include_target(line) {
                let base = path.parent().unwrap_or_else(|| Path::new("."));
                let previous = scan.scan_errors.len();
                parse_requirements_inner(&base.join(target), visited, scan);
                for error in &mut scan.scan_errors[previous..] {
                    error
                        .error
                        .push_str(&format!(" (included by {})", path.display()));
                }
            }
            continue;
        }
        if let Some((pkg, marker)) = extract_pep508_parts(line) {
            scan.dependencies.push(DeclaredDependency {
                package_name: pkg.clone(),
                normalized_name: normalize_package_name(&pkg),
                is_dev,
                is_optional: false,
                marker,
                source: DependencySource::Requirements(filename.clone()),
            });
        }
    }
}
