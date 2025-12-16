//! CytoScnPy analyzer module.
//!
//! This module contains the main analysis engine, broken down into:
//! - `types`: Result types (AnalysisResult, ParseError, AnalysisSummary)
//! - `heuristics`: Confidence adjustment functions
//! - `processing`: File processing and aggregation methods
//! - Core CytoScnPy struct and implementation

mod heuristics;
mod processing;
mod types;

// Re-export types for public API
pub use heuristics::{apply_heuristics, apply_penalties};
pub use types::{AnalysisResult, AnalysisSummary, ParseError};

use crate::config::Config;

/// The main analyzer struct.
/// Configuration options for the analysis are stored here.
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
    /// Whether to enable taint analysis.
    pub enable_taint: bool,
    /// Total number of files analyzed.
    pub total_files_analyzed: usize,
    /// Total number of lines analyzed.
    pub total_lines_analyzed: usize,
    /// Configuration object.
    pub config: Config,
}

impl Default for CytoScnPy {
    fn default() -> Self {
        Self {
            confidence_threshold: 60,
            enable_secrets: false,
            enable_danger: false,
            enable_quality: false,
            include_tests: true,
            exclude_folders: Vec::new(),
            include_folders: Vec::new(),
            include_ipynb: false,
            ipynb_cells: false,
            enable_taint: false,
            total_files_analyzed: 0,
            total_lines_analyzed: 0,
            config: Config::default(),
        }
    }
}

impl CytoScnPy {
    /// Creates a new `CytoScnPy` analyzer instance with the given configuration.
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    pub fn new(
        confidence_threshold: u8,
        enable_secrets: bool,
        enable_danger: bool,
        enable_quality: bool,
        include_tests: bool,
        exclude_folders: Vec<String>,
        include_folders: Vec<String>,
        include_ipynb: bool,
        ipynb_cells: bool,
        enable_taint: bool,
        config: Config,
    ) -> Self {
        Self {
            confidence_threshold,
            enable_secrets,
            enable_danger,
            enable_quality,
            include_tests,
            exclude_folders,
            include_folders,
            include_ipynb,
            ipynb_cells,
            enable_taint,
            total_files_analyzed: 0,
            total_lines_analyzed: 0,
            config,
        }
    }

    /// Builder-style method to set confidence threshold.
    #[must_use]
    pub fn with_confidence(mut self, threshold: u8) -> Self {
        self.confidence_threshold = threshold;
        self
    }

    /// Builder-style method to enable secrets scanning.
    #[must_use]
    pub fn with_secrets(mut self, enabled: bool) -> Self {
        self.enable_secrets = enabled;
        self
    }

    /// Builder-style method to enable danger scanning.
    #[must_use]
    pub fn with_danger(mut self, enabled: bool) -> Self {
        self.enable_danger = enabled;
        self
    }

    /// Builder-style method to enable quality scanning.
    #[must_use]
    pub fn with_quality(mut self, enabled: bool) -> Self {
        self.enable_quality = enabled;
        self
    }

    /// Builder-style method to include test files.
    #[must_use]
    pub fn with_tests(mut self, include: bool) -> Self {
        self.include_tests = include;
        self
    }

    /// Builder-style method to set excluded folders.
    #[must_use]
    pub fn with_excludes(mut self, folders: Vec<String>) -> Self {
        self.exclude_folders = folders;
        self
    }

    /// Builder-style method to set included folders.
    #[must_use]
    pub fn with_includes(mut self, folders: Vec<String>) -> Self {
        self.include_folders = folders;
        self
    }

    /// Builder-style method to include `IPython` notebooks.
    #[must_use]
    pub fn with_ipynb(mut self, include: bool) -> Self {
        self.include_ipynb = include;
        self
    }

    /// Builder-style method to enable cell-level reporting.
    #[must_use]
    pub fn with_ipynb_cells(mut self, enabled: bool) -> Self {
        self.ipynb_cells = enabled;
        self
    }

    /// Builder-style method to enable taint analysis.
    #[must_use]
    pub fn with_taint(mut self, enabled: bool) -> Self {
        self.enable_taint = enabled;
        self
    }

    /// Builder-style method to set config.
    #[must_use]
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    /// Counts the total number of Python files that would be analyzed.
    /// Useful for setting up a progress bar before analysis.
    pub fn count_files(&self, paths: &[std::path::PathBuf]) -> usize {
        use crate::constants::DEFAULT_EXCLUDE_FOLDERS;
        use walkdir::WalkDir;

        let mut count = 0;
        for path in paths {
            if path.is_file() {
                if path
                    .extension()
                    .is_some_and(|ext| ext == "py" || (self.include_ipynb && ext == "ipynb"))
                {
                    count += 1;
                }
            } else if path.is_dir() {
                let mut it = WalkDir::new(path).into_iter();
                while let Some(res) = it.next() {
                    if let Ok(entry) = res {
                        let name = entry.file_name().to_string_lossy();
                        let is_force_included = entry.file_type().is_dir()
                            && self.include_folders.iter().any(|f| f == &name);
                        let should_exclude = entry.file_type().is_dir()
                            && !is_force_included
                            && (DEFAULT_EXCLUDE_FOLDERS().iter().any(|&f| f == name)
                                || self.exclude_folders.iter().any(|f| f == &name));
                        if should_exclude {
                            it.skip_current_dir();
                            continue;
                        }
                        if entry.path().extension().is_some_and(|ext| {
                            ext == "py" || (self.include_ipynb && ext == "ipynb")
                        }) {
                            count += 1;
                        }
                    }
                }
            }
        }
        count
    }
}
