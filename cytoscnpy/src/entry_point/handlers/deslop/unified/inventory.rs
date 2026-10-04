//! Shared Python file inventory and scan completeness checks.

use rayon::prelude::*;
use serde::Serialize;
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
    pub sources: crate::utils::sources::SourceCache,
    pub definitions: crate::searchability::functions::ExtractedDefinitions,
    pub integrity: ScanIntegrity,
}

impl PythonInventory {
    pub fn collect(
        roots: &[PathBuf],
        excludes: &[String],
        include_tests: bool,
        verbose: bool,
    ) -> Self {
        let (mut files, walk_errors) =
            crate::commands::utils::find_python_files_with_options_and_issues(
                roots,
                excludes,
                &[],
                include_tests,
                verbose,
            );
        files.sort();
        files.dedup();
        let sources: crate::utils::sources::SourceCache = files
            .par_iter()
            .map(|path| {
                let source = crate::utils::sources::load_source(path, None)
                    .map(std::borrow::Cow::into_owned);
                (path.clone(), source)
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
        for path in &files {
            match &sources[path] {
                Ok(source) => {
                    let found = crate::searchability::functions::extract_definitions_from_ast(
                        &source.module,
                        &source.content,
                        path,
                    );
                    definitions.functions.extend(found.functions);
                    definitions.classes.extend(found.classes);
                    checked += 1;
                }
                Err(reason) => issues.push(ScanIssue {
                    path: path.clone(),
                    reason: reason.clone(),
                }),
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
            sources,
            definitions,
            integrity,
        }
    }
}
