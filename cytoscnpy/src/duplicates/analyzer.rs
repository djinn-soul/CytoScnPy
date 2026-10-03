//! Analyzer for duplicate code clusters and non-overlapping line calculations.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::interval::count_non_overlapping_lines;
use super::types::{
    DuplicateCluster, DuplicateLocation, DuplicatesResult, DuplicatesStats, FileDuplicateStats,
};
use crate::clones::{CloneConfig, CloneDetector};
use crate::commands::utils::find_python_files;

/// Options controlling duplicate code analysis.
#[derive(Debug, Clone)]
pub struct DuplicatesOptions {
    /// Minimum similarity threshold (0.0 - 1.0).
    pub min_similarity: f64,
    /// Minimum line threshold for code fragments.
    pub min_lines: usize,
    /// Whether to include test files.
    pub include_tests: bool,
}

impl Default for DuplicatesOptions {
    fn default() -> Self {
        Self {
            min_similarity: 0.85,
            min_lines: 4,
            include_tests: false,
        }
    }
}

/// Analyzes duplicate code clusters and non-overlapping line totals across `roots`.
#[must_use]
pub fn analyze_duplicates(
    roots: &[PathBuf],
    exclude: &[String],
    options: &DuplicatesOptions,
    verbose: bool,
) -> DuplicatesResult {
    let python_files = find_python_files(roots, exclude, verbose);
    let mut result = analyze_duplicates_files(&python_files, options);
    result.roots = roots.to_vec();
    result
}

/// Analyzes duplicate code clusters across an explicit list of Python files.
#[must_use]
pub fn analyze_duplicates_files(
    files: &[PathBuf],
    options: &DuplicatesOptions,
) -> DuplicatesResult {
    let valid_files: Vec<PathBuf> = files
        .iter()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e == "py" || e == "pyi")
                && (options.include_tests || !crate::utils::is_test_path(&p.to_string_lossy()))
                && !crate::utils::is_likely_minified(p, None)
        })
        .cloned()
        .collect();

    // Map each file to its physical line count
    let mut file_line_counts: HashMap<PathBuf, usize> = HashMap::new();
    let mut total_scanned_lines = 0usize;

    for path in &valid_files {
        let lines = fs::read_to_string(path)
            .map(|content| content.lines().count())
            .unwrap_or(0);
        file_line_counts.insert(path.clone(), lines);
        total_scanned_lines += lines;
    }

    // Configure and run clone detector
    let config = CloneConfig {
        min_similarity: options.min_similarity,
        min_lines: options.min_lines,
        include_tests: options.include_tests,
        ..Default::default()
    };

    let detector = CloneDetector::with_config(config).unwrap_or_else(|_| CloneDetector::new());
    let clone_result = detector.detect_from_paths(&valid_files);

    // Build duplicate clusters and collect per-file intervals
    let mut clusters = Vec::new();
    let mut file_intervals: HashMap<PathBuf, Vec<(usize, usize)>> = HashMap::new();
    let mut file_instance_counts: HashMap<PathBuf, usize> = HashMap::new();
    let mut exact_clusters = 0;
    let mut renamed_clusters = 0;
    let mut similar_clusters = 0;

    for (idx, group) in clone_result.groups.into_iter().enumerate() {
        let cluster_id = idx + 1;
        let clone_type_str = group.clone_type.short_name().to_owned();

        match group.clone_type {
            crate::clones::CloneType::Type1 => exact_clusters += 1,
            crate::clones::CloneType::Type2 => renamed_clusters += 1,
            crate::clones::CloneType::Type3 => similar_clusters += 1,
        }

        let canonical_lines = group
            .canonical()
            .map(|c| c.end_line.saturating_sub(c.start_line) + 1)
            .unwrap_or(0);

        let mut locations = Vec::new();
        for inst in &group.instances {
            file_intervals
                .entry(inst.file.clone())
                .or_default()
                .push((inst.start_line, inst.end_line));

            *file_instance_counts.entry(inst.file.clone()).or_insert(0) += 1;

            locations.push(DuplicateLocation {
                file: inst.file.clone(),
                start_line: inst.start_line,
                end_line: inst.end_line,
                name: inst.name.clone(),
                node_kind: format!("{:?}", inst.node_kind).to_lowercase(),
            });
        }

        clusters.push(DuplicateCluster {
            id: cluster_id,
            clone_type: clone_type_str,
            similarity: group.avg_similarity,
            line_count: canonical_lines,
            locations,
        });
    }

    // Calculate per-file non-overlapping duplicate lines
    let mut file_stats = Vec::new();
    let mut total_duplicate_lines = 0usize;

    for (path, intervals) in file_intervals {
        let duplicate_lines = count_non_overlapping_lines(intervals);
        total_duplicate_lines += duplicate_lines;

        let total_lines = file_line_counts.get(&path).copied().unwrap_or(0);
        let duplicate_pct = if total_lines > 0 {
            (duplicate_lines as f64 / total_lines as f64) * 100.0
        } else {
            0.0
        };

        let instance_count = file_instance_counts.get(&path).copied().unwrap_or(0);

        file_stats.push(FileDuplicateStats {
            file: path,
            duplicate_lines,
            total_lines,
            duplicate_pct,
            instance_count,
        });
    }

    file_stats.sort_by_key(|a| std::cmp::Reverse(a.duplicate_lines));

    let overall_duplicate_pct = if total_scanned_lines > 0 {
        (total_duplicate_lines as f64 / total_scanned_lines as f64) * 100.0
    } else {
        0.0
    };

    let stats = DuplicatesStats {
        cluster_count: clusters.len(),
        total_duplicate_lines,
        total_scanned_lines,
        duplicate_pct: overall_duplicate_pct,
        affected_files: file_stats.len(),
        exact_clusters,
        renamed_clusters,
        similar_clusters,
    };

    DuplicatesResult {
        clusters,
        stats,
        file_stats,
        files_scanned: valid_files.len(),
        roots: Vec::new(),
    }
}
