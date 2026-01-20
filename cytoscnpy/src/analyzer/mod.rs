//! CytoScnPy analyzer module.
//!
//! This module contains the main analysis engine, broken down into:
//! - `types`: Result types (AnalysisResult, ParseError, AnalysisSummary)
//! - `heuristics`: Confidence adjustment functions
//! - `processing`: File processing and aggregation methods
//! - Core CytoScnPy struct and implementation

mod aggregation;
mod builder;
mod heuristics;
pub mod single_file;
mod traversal;
mod utils;

/// Result types and analysis summaries.
pub mod types;

// Re-export types for public API
pub use heuristics::{apply_heuristics, apply_penalties};
pub use types::{AnalysisResult, AnalysisSummary, ParseError};

use crate::config::Config;

/// The main analyzer struct.
/// Configuration options for the analysis are stored here.
#[allow(clippy::struct_excessive_bools)]
pub struct CytoScnPy {
    /// Confidence threshold (0-100). Findings below this are ignored.
    pub confidence_threshold: u8,
    /// Whether to scan for secrets.
    pub enable_secrets: bool,
    /// Whether to scan for dangerous code.
    pub enable_danger: bool,
    /// Whether to scan for quality issues.
    pub enable_quality: bool,
    /// Whether to include test files in the analysis.
    pub include_tests: bool,
    /// Folders to exclude from analysis.
    pub exclude_folders: Vec<String>,
    /// Folders to force-include in analysis (overrides default exclusions).
    pub include_folders: Vec<String>,
    /// Whether to include `IPython` notebooks in analysis.
    pub include_ipynb: bool,
    /// Whether to report findings at cell level for notebooks.
    pub ipynb_cells: bool,
    /// Total number of files analyzed.
    pub total_files_analyzed: usize,
    /// Total number of lines analyzed.
    pub total_lines_analyzed: usize,
    /// Configuration object.
    pub config: Config,
    /// Debug delay in milliseconds (for testing progress bar).
    pub debug_delay_ms: Option<u64>,
    /// Progress bar for tracking analysis progress (thread-safe).
    pub progress_bar: Option<std::sync::Arc<indicatif::ProgressBar>>,
    /// Whether to enable verbose logging.
    pub verbose: bool,
    /// Analysis root for path containment and relative resolution.
    pub analysis_root: std::path::PathBuf,
}

impl Default for CytoScnPy {
    fn default() -> Self {
        Self {
            confidence_threshold: 60,
            enable_secrets: false,
            enable_danger: false,
            enable_quality: false,
            include_tests: false,
            exclude_folders: Vec::new(),
            include_folders: Vec::new(),
            include_ipynb: false,
            ipynb_cells: false,
            total_files_analyzed: 0,
            total_lines_analyzed: 0,
            config: Config::default(),
            debug_delay_ms: None,
            progress_bar: None,
            verbose: false,
            analysis_root: std::path::PathBuf::from("."),
        }
    }
}
