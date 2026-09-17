//! Duplicate-code clusters and non-overlapping duplicate-line totals.
//!
//! Provides analysis of mutual duplicate code clusters, interval-based
//! non-overlapping physical line counting per file and project-wide,
//! and terminal/JSON reporting.

pub mod analyzer;
pub mod interval;
pub mod reporter;
pub mod types;

#[cfg(test)]
mod tests;

pub use analyzer::{analyze_duplicates, analyze_duplicates_files, DuplicatesOptions};
pub use interval::{count_non_overlapping_lines, merge_intervals};
pub use reporter::{print_json_report, print_terminal_report};
pub use types::{
    DuplicateCluster, DuplicateLocation, DuplicatesResult, DuplicatesStats, FileDuplicateStats,
};
