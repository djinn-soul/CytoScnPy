//! Repository health, setup reliability, and configuration metadata scanner.
//!
//! Inspects:
//! - Tooling configuration (formatters, linters, type checkers, tests, CI, Docker, lockfiles)
//! - Python project declaration in `pyproject.toml`
//! - Polyglot language breakdown and test-to-source ratios
//! - Weighted setup reliability scoring (0-100) and actionable setup guidance

/// Module for aggregating doctor health results across scan targets.
pub mod aggregation;
/// Module for detecting repository configuration files and tooling.
pub mod config_detector;
mod config_extras;
/// Module for polyglot source language and configuration format detection.
pub mod language;
/// Module for deep inspection of `pyproject.toml`.
pub mod pyproject;
/// Module for calculating setup reliability scores and recommendations.
pub mod reliability;
/// Module for terminal tables and JSON output reporting.
pub mod reporter;
/// Module for gitignore-aware repository structure and language volume scanning.
pub mod structure;
/// Module defining data structures and types for repository health analysis.
pub mod types;

#[cfg(test)]
mod tests;

pub use aggregation::*;
pub use config_detector::*;
pub use language::{detect_language, SupportedLanguage};
pub use pyproject::*;
pub use reliability::*;
pub use reporter::{print_json_report, print_terminal_report};
pub use structure::*;
pub use types::*;

use std::path::Path;

/// Runs repository health and setup reliability analysis on the target path.
#[must_use]
pub fn run_doctor(repo_root: &Path, config: &DoctorConfig) -> DoctorResult {
    let mut configs = detect_configurations(repo_root);

    let pyproject = inspect_pyproject(repo_root);
    if let Some(ref p) = pyproject {
        supplement_configs_with_pyproject(&mut configs, p, &repo_root.join("pyproject.toml"));
    }

    let structure = scan_repo_structure(repo_root, &config.excludes);
    let reliability = evaluate_setup_reliability(repo_root, &configs);

    DoctorResult {
        root_path: repo_root.to_path_buf(),
        configs,
        pyproject,
        structure,
        reliability,
    }
}

/// Resolves file and subdirectory targets to their nearest project boundary.
#[must_use]
pub fn resolve_doctor_target(path: &Path, fallback: &Path) -> std::path::PathBuf {
    let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let directory = if path.is_file() {
        resolved.parent().unwrap_or(fallback)
    } else {
        resolved.as_path()
    };
    let project = crate::utils::discover_project_root(directory);
    if project != directory
        || directory.join("pyproject.toml").is_file()
        || directory.join("setup.py").is_file()
        || directory.join("setup.cfg").is_file()
        || directory.join(".cytoscnpy.toml").is_file()
        || directory.join(".git").exists()
    {
        return project;
    }
    let fallback = fallback
        .canonicalize()
        .unwrap_or_else(|_| fallback.to_path_buf());
    if directory.starts_with(&fallback) {
        fallback
    } else {
        directory.to_path_buf()
    }
}
