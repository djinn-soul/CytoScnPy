use crate::halstead::HalsteadMetrics;
use crate::raw_metrics::RawMetrics;
use crate::rules::secrets::SecretFinding;
use crate::rules::Finding;
use crate::taint::types::TaintFinding;
use crate::visitor::Definition;
use serde::Serialize;

/// Represents a parsing error in a file.
#[derive(Serialize, Clone)]
pub struct ParseError {
    /// The file where the error occurred.
    pub file: std::path::PathBuf,
    /// The error message.
    pub error: String,
}

/// Metrics for a single file.
#[derive(Serialize, Clone)]
pub struct FileMetrics {
    /// The file path.
    pub file: std::path::PathBuf,
    /// Lines of Code.
    pub loc: usize,
    /// Source Lines of Code.
    pub sloc: usize,
    /// Cyclomatic Complexity.
    pub complexity: f64,
    /// Maintainability Index.
    pub mi: f64,
    /// Number of issues found.
    pub total_issues: usize,
}

/// Holds the results of the analysis.
/// This struct is serialized to JSON if requested.
#[derive(Serialize)]
pub struct AnalysisResult {
    /// List of functions that were defined but never used.
    pub unused_functions: Vec<Definition>,
    /// List of methods that were defined but never used.
    pub unused_methods: Vec<Definition>,
    /// List of imports that were imported but never used.
    pub unused_imports: Vec<Definition>,
    /// List of classes that were defined but never used.
    pub unused_classes: Vec<Definition>,
    /// List of variables that were defined but never used.
    pub unused_variables: Vec<Definition>,
    /// List of parameters that were defined but never used.
    pub unused_parameters: Vec<Definition>,
    /// List of discovered secrets (e.g., API keys).
    pub secrets: Vec<SecretFinding>,
    /// List of security vulnerabilities found.
    pub danger: Vec<Finding>,
    /// List of code quality issues found.
    pub quality: Vec<Finding>,
    /// List of taint analysis findings (data flow vulnerabilities).
    pub taint_findings: Vec<TaintFinding>,
    /// List of parse errors encountered.
    pub parse_errors: Vec<ParseError>,
    /// Per-file metrics.
    pub file_metrics: Vec<FileMetrics>,
    /// Summary statistics of the analysis.
    pub analysis_summary: AnalysisSummary,
}

/// Summary statistics for the analysis result.
#[derive(Serialize, Clone)]
pub struct AnalysisSummary {
    /// Total number of files scanned.
    pub total_files: usize,
    /// Total number of secrets found.
    pub secrets_count: usize,
    /// Total number of dangerous patterns found.
    pub danger_count: usize,
    /// Total number of quality issues found.
    pub quality_count: usize,
    /// Total number of taint findings.
    pub taint_count: usize,
    /// Total number of parse errors found.
    pub parse_errors_count: usize,
    /// Total number of lines analyzed.
    pub total_lines_analyzed: usize,
    /// Total number of definitions found (for percentage calculation).
    pub total_definitions: usize,
    /// Average Cyclomatic Complexity across all functions/files.
    pub average_complexity: f64,
    /// Average Maintainability Index across all files.
    pub average_mi: f64,
    /// Total number of directories scanned.
    pub total_directories: usize,
    /// Total size of analyzed code in KB.
    pub total_size: f64,
    /// Total number of function definitions found.
    pub functions_count: usize,
    /// Total number of class definitions found.
    pub classes_count: usize,
    /// Aggregated Raw Metrics (LOC, SLOC, Comments, etc.).
    pub raw_metrics: RawMetrics,
    /// Aggregated Halstead Metrics (Volume, Effort, etc.).
    pub halstead_metrics: HalsteadMetrics,
}
