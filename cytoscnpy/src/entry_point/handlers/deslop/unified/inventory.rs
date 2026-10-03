//! Shared Python file inventory and scan completeness checks.

use rayon::prelude::*;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize)]
pub(super) struct ScanIssue {
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Serialize)]
pub(super) struct ScanIntegrity {
    pub files_discovered: usize,
    pub files_checked: usize,
    pub complete: bool,
    pub issues: Vec<ScanIssue>,
}

pub(super) struct PythonInventory {
    pub files: Vec<PathBuf>,
    pub definitions: crate::searchability::functions::ExtractedDefinitions,
    pub integrity: ScanIntegrity,
}

impl PythonInventory {
    pub fn collect(roots: &[PathBuf], excludes: &[String], verbose: bool) -> Self {
        let (mut files, walk_errors) =
            crate::commands::utils::find_python_files_with_issues(roots, excludes, verbose);
        files.sort();
        files.dedup();
        let scanned: Vec<_> = files
            .par_iter()
            .map(|path| {
                let source = fs::read_to_string(path).map_err(|error| ScanIssue {
                    path: path.clone(),
                    reason: format!("read error: {error}"),
                })?;
                let parsed =
                    ruff_python_parser::parse_module(&source).map_err(|error| ScanIssue {
                        path: path.clone(),
                        reason: format!("Python parse error: {error}"),
                    })?;
                Ok(
                    crate::searchability::functions::extract_definitions_from_ast(
                        &parsed.into_syntax(),
                        &source,
                        path,
                    ),
                )
            })
            .collect();
        let mut issues: Vec<_> = walk_errors
            .into_iter()
            .map(|(path, reason)| ScanIssue {
                path,
                reason: format!("file discovery error: {reason}"),
            })
            .collect();
        let mut checked = 0;
        let mut definitions = crate::searchability::functions::ExtractedDefinitions::default();
        for result in scanned {
            match result {
                Ok(found) => {
                    definitions.functions.extend(found.functions);
                    definitions.classes.extend(found.classes);
                    checked += 1;
                }
                Err(issue) => issues.push(issue),
            }
        }
        let integrity = ScanIntegrity {
            files_discovered: files.len(),
            files_checked: checked,
            complete: issues.is_empty(),
            issues,
        };
        Self {
            files,
            definitions,
            integrity,
        }
    }

    pub fn check_additional(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        let known: BTreeSet<_> = self.files.iter().collect();
        let additional: BTreeSet<_> = paths.into_iter().collect();
        for path in additional {
            if known.contains(&path) {
                continue;
            }
            self.integrity.files_discovered += 1;
            match fs::read_to_string(&path) {
                Ok(_) => self.integrity.files_checked += 1,
                Err(error) => self.integrity.issues.push(ScanIssue {
                    path,
                    reason: format!("read error: {error}"),
                }),
            }
        }
        self.integrity.complete = self.integrity.issues.is_empty();
    }
}
