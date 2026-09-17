//! Types and data models for duplicate code clusters and line totals.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single location where a duplicated code fragment appears.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateLocation {
    /// File containing the duplicated fragment.
    pub file: PathBuf,
    /// 1-indexed starting line.
    pub start_line: usize,
    /// 1-indexed ending line (inclusive).
    pub end_line: usize,
    /// Name of the enclosing function, class, or method, if known.
    pub name: Option<String>,
    /// Kind of code element (e.g. "function", "method", "class").
    pub node_kind: String,
}

impl DuplicateLocation {
    /// Number of lines spanned by this location.
    #[must_use]
    pub const fn line_span(&self) -> usize {
        self.end_line.saturating_sub(self.start_line) + 1
    }
}

/// A cluster of mutually duplicate code fragments across the codebase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DuplicateCluster {
    /// 1-indexed cluster identifier.
    pub id: usize,
    /// Clone category: "Exact", "Renamed", or "Similar".
    pub clone_type: String,
    /// Average similarity score within the cluster (0.0 - 1.0).
    pub similarity: f64,
    /// Line count of the canonical instance in this cluster.
    pub line_count: usize,
    /// All locations in this cluster (2 or more).
    pub locations: Vec<DuplicateLocation>,
}

/// Duplication statistics for an individual source file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileDuplicateStats {
    /// File path.
    pub file: PathBuf,
    /// Total non-overlapping duplicated lines in this file.
    pub duplicate_lines: usize,
    /// Total physical lines in this file.
    pub total_lines: usize,
    /// Ratio of duplicated lines to total lines (0.0 - 100.0).
    pub duplicate_pct: f64,
    /// Count of duplicate instances found in this file.
    pub instance_count: usize,
}

/// Aggregate duplicate code statistics for the scanned project.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DuplicatesStats {
    /// Total number of duplicate clusters.
    pub cluster_count: usize,
    /// Total non-overlapping duplicate lines across all files.
    pub total_duplicate_lines: usize,
    /// Total lines scanned across all target files.
    pub total_scanned_lines: usize,
    /// Overall duplicate line percentage (0.0 - 100.0).
    pub duplicate_pct: f64,
    /// Number of distinct files containing at least one duplicate instance.
    pub affected_files: usize,
    /// Count of Type-1 (Exact Copy) clusters.
    pub exact_clusters: usize,
    /// Count of Type-2 (Renamed Copy) clusters.
    pub renamed_clusters: usize,
    /// Count of Type-3 (Similar Code) clusters.
    pub similar_clusters: usize,
}

/// Overall results of duplicate-code cluster and line analysis.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DuplicatesResult {
    /// All detected duplicate clusters.
    pub clusters: Vec<DuplicateCluster>,
    /// Project-wide aggregate statistics.
    pub stats: DuplicatesStats,
    /// Per-file duplication summaries.
    pub file_stats: Vec<FileDuplicateStats>,
    /// Number of Python files scanned.
    pub files_scanned: usize,
    /// Root paths analyzed.
    pub roots: Vec<PathBuf>,
}

impl DuplicatesResult {
    /// Returns `true` if no duplicate clusters were found.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.clusters.is_empty()
    }
}
