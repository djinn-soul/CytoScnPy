//! Checked discovery and source loading shared by metric commands.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub(super) fn discover_files(
    roots: &[PathBuf],
    exclude: &[String],
    include_tests: bool,
    verbose: bool,
) -> Result<Vec<PathBuf>> {
    let (files, issues) = super::utils::find_python_files_with_options_and_issues(
        roots,
        exclude,
        &[],
        include_tests,
        verbose,
    );
    anyhow::ensure!(
        issues.is_empty(),
        "Metric file discovery failed: {}",
        issues
            .iter()
            .map(|(path, error)| format!("{}: {error}", path.display()))
            .collect::<Vec<_>>()
            .join("; ")
    );
    Ok(files)
}

pub(super) fn read_source(path: &Path) -> Result<String> {
    anyhow::ensure!(
        !crate::CANCELLED.load(std::sync::atomic::Ordering::Relaxed),
        "Metric scan cancelled"
    );
    std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Python source {}", path.display()))
}

pub(super) fn parse_source(source: &str, path: &Path) -> Result<ruff_python_ast::ModModule> {
    ruff_python_parser::parse_module(source)
        .map(ruff_python_parser::Parsed::into_syntax)
        .with_context(|| format!("Failed to parse Python source {}", path.display()))
}
