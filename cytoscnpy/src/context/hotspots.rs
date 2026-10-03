use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::git_scanner::relativize_path;
use super::types::{HotspotFile, HotspotRiskLevel};
use crate::complexity::calculate_module_complexity;

/// Evaluates risk level based on churn and cyclomatic complexity.
#[must_use]
pub fn classify_hotspot_risk(commit_count: usize, complexity: usize) -> HotspotRiskLevel {
    let score = commit_count * complexity;
    if score >= 100 || (commit_count >= 10 && complexity >= 20) {
        HotspotRiskLevel::Critical
    } else if score >= 40 || (commit_count >= 5 && complexity >= 10) {
        HotspotRiskLevel::High
    } else if score >= 15 || (commit_count >= 3 && complexity >= 6) {
        HotspotRiskLevel::Medium
    } else {
        HotspotRiskLevel::Low
    }
}

/// Discovers high-churn, complex files ("hotspots") across the codebase.
pub fn find_hotspots<S: std::hash::BuildHasher>(
    repo_path: &Path,
    file_paths: &[PathBuf],
    file_commits: &HashMap<PathBuf, usize, S>,
) -> Vec<HotspotFile> {
    let mut hotspots = Vec::new();

    for file_path in file_paths {
        let commit_count = file_commits.get(file_path).copied().unwrap_or(0);
        if commit_count == 0 {
            continue;
        }

        let Ok(content) = fs::read_to_string(file_path) else {
            continue;
        };

        let complexity = calculate_module_complexity(&content).unwrap_or(1);
        let risk_score = commit_count * complexity;
        let risk_level = classify_hotspot_risk(commit_count, complexity);

        let display_path = relativize_path(repo_path, file_path).display().to_string();

        hotspots.push(HotspotFile {
            path: file_path.clone(),
            display_path,
            commit_count,
            cyclomatic_complexity: complexity,
            risk_score,
            risk_level,
        });
    }

    hotspots.sort_by(|a, b| {
        b.risk_score
            .cmp(&a.risk_score)
            .then_with(|| b.cyclomatic_complexity.cmp(&a.cyclomatic_complexity))
    });

    hotspots
}

/// Checks if any hotspots are considered Critical or High risk.
#[must_use]
pub fn has_severe_hotspots(hotspots: &[HotspotFile]) -> bool {
    hotspots.iter().any(|h| {
        matches!(
            h.risk_level,
            HotspotRiskLevel::Critical | HotspotRiskLevel::High
        )
    })
}
