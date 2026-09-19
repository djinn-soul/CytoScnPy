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

/// Resolves a target path consistently to a directory for repository health analysis.
/// For file targets, resolves to the parent directory (using canonicalization when available).
#[must_use]
pub fn resolve_doctor_target(path: &Path, fallback: &Path) -> std::path::PathBuf {
    if path.is_file() {
        if let Ok(abs) = path.canonicalize() {
            if let Some(parent) = abs.parent() {
                return parent.to_path_buf();
            }
        }
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                return parent.to_path_buf();
            }
        }
        fallback.to_path_buf()
    } else if let Ok(abs) = path.canonicalize() {
        abs
    } else {
        path.to_path_buf()
    }
}
