use serde::Serialize;

/// File-level metrics used by stats and files commands.
#[derive(Serialize, Clone)]
pub(super) struct FileMetrics {
    pub(super) file: String,
    pub(super) code_lines: usize,
    pub(super) comment_lines: usize,
    pub(super) empty_lines: usize,
    pub(super) total_lines: usize,
    pub(super) size_kb: f64,
}

/// Serializable report payload for stats output.
#[derive(Serialize)]
pub(super) struct StatsReport {
    pub(super) total_files: usize,
    pub(super) total_directories: usize,
    pub(super) total_size_kb: f64,
    pub(super) total_lines: usize,
    pub(super) code_lines: usize,
    pub(super) comment_lines: usize,
    pub(super) empty_lines: usize,
    pub(super) total_functions: usize,
    pub(super) total_classes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) files: Option<Vec<FileMetrics>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) secrets: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) danger: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) quality: Option<Vec<String>>,
}

/// Aggregated project stats for reporting.
pub(super) struct ProjectStats {
    pub(super) total_files: usize,
    pub(super) total_directories: usize,
    pub(super) total_size_kb: f64,
    pub(super) total_lines: usize,
    pub(super) code_lines: usize,
    pub(super) comment_lines: usize,
    pub(super) empty_lines: usize,
    pub(super) total_functions: usize,
    pub(super) total_classes: usize,
    pub(super) file_metrics: Vec<FileMetrics>,
}

/// Flags for enabling specific inspection types.
#[derive(Serialize, Clone, Copy, Debug, Default)]
pub struct Inspections {
    /// Include secrets findings.
    pub secrets: bool,
    /// Include dangerous pattern findings.
    pub danger: bool,
    /// Include code quality findings.
    pub quality: bool,
}

/// Options for scanning during stats analysis.
#[derive(Serialize, Clone, Copy, Debug, Default)]
pub struct ScanOptions {
    /// Include all file-level and finding metrics.
    pub all: bool,
    /// Inspections flags.
    pub inspections: Inspections,
    /// Return output as JSON.
    pub json: bool,
}

impl ScanOptions {
    /// Checks if any analysis mode is enabled.
    #[must_use]
    pub fn is_any_enabled(self) -> bool {
        self.all || self.inspections.secrets || self.inspections.danger || self.inspections.quality
    }

    /// Whether to include secrets in the scan.
    #[must_use]
    pub fn include_secrets(self) -> bool {
        self.all || self.inspections.secrets
    }

    /// Whether to include dangerous patterns in the scan.
    #[must_use]
    pub fn include_danger(self) -> bool {
        self.all || self.inspections.danger
    }

    /// Whether to include quality issues in the scan.
    #[must_use]
    pub fn include_quality(self) -> bool {
        self.all || self.inspections.quality
    }
}
