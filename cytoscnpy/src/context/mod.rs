//! Git-aware context, code churn, and LLM token budget analysis.
//!
//! Provides insights into:
//! - Active vs frozen code surface based on recent Git history
//! - Code churn and high-frequency "hot files"
//! - Churn-complexity "hotspots" (files with both high change frequency and high cyclomatic complexity)
//! - LLM token navigation budget and context capacity headroom

/// Module for estimating LLM token costs and context budgets.
pub mod estimator;
/// Module for scanning Git commit history and classifying active vs frozen files.
pub mod git_scanner;
/// Module for detecting high-churn, complex hotspot files.
pub mod hotspots;
/// Module for printing terminal tables and JSON reports.
pub mod reporter;
/// Module defining data types and structures for context analysis.
pub mod types;

#[cfg(test)]
mod tests;

pub use estimator::*;
pub use git_scanner::*;
pub use hotspots::*;
pub use reporter::{print_json_report, print_terminal_report};
pub use types::*;

use crate::commands::utils::find_python_files;
use std::path::{Path, PathBuf};

/// Analyzes codebase context, Git churn, hotspots, and LLM token budgets.
#[must_use]
pub fn analyze_context(
    roots: &[PathBuf],
    exclude: &[String],
    config: &ContextConfig,
) -> ContextAnalysisResult {
    let file_paths = find_python_files(roots, exclude, config.verbose);
    let repo_root = roots
        .first()
        .map_or_else(|| Path::new("."), |p| p.as_path());

    let scanned_infos: Vec<ScannedFileInfo> = file_paths
        .iter()
        .filter_map(|p| collect_file_info(p))
        .collect();

    let (total_lines, total_bytes) = scanned_infos
        .iter()
        .fold((0, 0u64), |(lines, bytes), file| {
            (lines + file.lines, bytes + file.bytes)
        });

    let (git_activity, file_commits) =
        scan_git_activity(repo_root, &scanned_infos, config.git_months, config.no_git);

    let hotspots = find_hotspots(repo_root, &file_paths, &file_commits);

    let avg_complexity = if hotspots.is_empty() {
        3.0
    } else {
        let total_c: usize = hotspots.iter().map(|h| h.cyclomatic_complexity).sum();
        total_c as f64 / hotspots.len() as f64
    };

    let token_budget = compute_token_budget(
        scanned_infos.len(),
        total_lines,
        avg_complexity,
        2.5,
        hotspots.len(),
        config.context_budget,
    );

    ContextAnalysisResult {
        total_files: scanned_infos.len(),
        total_lines,
        total_bytes,
        git_activity,
        hotspots,
        token_budget,
    }
}
