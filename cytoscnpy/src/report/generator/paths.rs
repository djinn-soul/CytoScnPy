//! One deterministic registry gives every source path a distinct report filename.

use crate::analyzer::AnalysisResult;
use crate::report::templates::IssueItem;
use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct ReportPaths {
    filenames: BTreeMap<String, String>,
}

impl ReportPaths {
    pub(super) fn new(result: &AnalysisResult, issues: &[IssueItem]) -> Self {
        let mut paths: BTreeSet<String> = result
            .file_metrics
            .iter()
            .map(|metric| metric.file.to_string_lossy().into_owned())
            .collect();
        paths.extend(issues.iter().map(|issue| issue.file.clone()));
        for clone in &result.clones {
            paths.insert(clone.file.to_string_lossy().into_owned());
            paths.insert(clone.related_clone.file.to_string_lossy().into_owned());
        }
        let filenames = paths
            .into_iter()
            .enumerate()
            .map(|(index, path)| (path, format!("file-{index:06}.html")))
            .collect();
        Self { filenames }
    }

    pub(super) fn filename(&self, file: &str) -> Result<&str> {
        self.filenames
            .get(file)
            .map(String::as_str)
            .with_context(|| format!("Source path is missing from HTML report registry: {file}"))
    }
}
