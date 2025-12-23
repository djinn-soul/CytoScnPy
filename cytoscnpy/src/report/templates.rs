use askama::Template;
use serde::{Deserialize, Serialize};

/// Score breakdown for a specific category (e.g., Complexity, Security).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CategoryScore {
    /// Category name.
    pub name: String,
    /// Numerical score (0-100).
    pub score: u8,
    /// Number of issues contributing to this score (if applicable).
    pub issue_count: usize,
    /// Letter grade (A-F).
    pub grade: String,
    /// Color hex for the grade.
    pub color: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Represents the overall quality score of a project.
pub struct OverallScore {
    /// The total numerical score (0-100).
    pub total_score: u8,
    /// The overall letter grade (A-F).
    pub grade: String,
    /// Breakdown of scores by category.
    pub categories: Vec<CategoryScore>,
}

#[derive(Debug, Clone)]
/// Formatted Halstead metrics for display.
pub struct FormattedHalsteadMetrics {
    /// Program vocabulary size.
    pub volume: String,
    /// Qualitative level for volume.
    pub volume_level: String,
    /// Color hex for volume level.
    pub volume_color: String,
    /// Icon for volume level.
    pub volume_icon: String,
    /// Difficulty level to read/maintain.
    pub difficulty: String,
    /// Qualitative level for difficulty.
    pub difficulty_level: String,
    /// Color hex for difficulty level.
    pub difficulty_color: String,
    /// Icon for difficulty level.
    pub difficulty_icon: String,
    /// Estimated effort to implement.
    pub effort: String,
    /// Qualitative level for effort.
    pub effort_level: String,
    /// Color hex for effort level.
    pub effort_color: String,
    /// Icon for effort level.
    pub effort_icon: String,
    /// Estimated number of delivered bugs.
    pub bugs: String,
    /// Qualitative level for bugs.
    pub bugs_level: String,
    /// Color hex for bugs level.
    pub bugs_color: String,
    /// Icon for bugs level.
    pub bugs_icon: String,
    /// Estimated time to implement.
    pub time: String,
    /// Calculated program length.
    pub calculated_length: String,
}

#[derive(Debug, Clone)]
/// View model for file-specific metrics used in the Files report page.
pub struct FileMetricsView {
    /// The relative file path.
    pub file: String,
    /// Source Lines of Code.
    pub sloc: usize,
    /// Cyclomatic Complexity.
    pub complexity: f64,
    /// Raw Maintainability Index (unclamped).
    pub raw_mi: f64,
    /// Maintainability Index formatted as a string.
    pub mi: String,
    /// Total number of issues found in the file.
    pub total_issues: usize,
    /// Link to the detailed file view.
    pub link: String,
}

/// View model for the main dashboard page.
#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    /// The overall project score breakdown.
    pub score: OverallScore,
    /// Color hex for the score grade.
    pub score_color: String,
    /// Total number of files analyzed.
    pub total_files: usize,
    /// Total lines of code.
    pub total_lines: usize,
    /// Total issues found.
    pub total_issues: usize,
    /// Color for total issues count.
    pub total_issues_color: String,
    /// Unused imports count.
    pub unused_imports: usize,
    /// Unused functions count.
    pub unused_functions: usize,
    /// Unused classes count.
    pub unused_classes: usize,
    /// Unused variables count.
    pub unused_variables: usize,
    /// Unused methods count.
    pub unused_methods: usize,
    /// Unused parameters count.
    pub unused_parameters: usize,
    /// Average MI formatted string.
    pub average_mi_str: String,
    /// Color hex for the average MI.
    pub average_mi_color: String,
    /// Comprehensive analysis summary.
    pub summary: crate::analyzer::AnalysisSummary,
    /// Halstead metrics view.
    pub halstead_view: FormattedHalsteadMetrics,
    /// Generation timestamp.
    pub generated_at: String,
    /// CytoScnPy version.
    pub version: String,
    /// Root path for navigation (e.g. "." or "..")
    pub root_path: String,
}

/// View model for the issues report page.
#[derive(Template)]
#[template(path = "issues.html")]
pub struct IssuesTemplate {
    /// List of unused code issues.
    pub unused_code: Vec<IssueItem>,
    /// List of security issues.
    pub securityable: Vec<IssueItem>,
    /// List of quality issues.
    pub quality: Vec<IssueItem>,
    /// Generation timestamp.
    pub generated_at: String,
    /// CytoScnPy version.
    pub version: String,
    /// Root path for navigation (e.g. "." or "..")
    pub root_path: String,
}

#[derive(Debug, Clone, Serialize)]
/// Represents a single issue to be displayed in the report.
pub struct IssueItem {
    /// Issue category (e.g., Unused, Security).
    pub category: String, // Unused, Security, Quality
    /// Issue severity (HIGH, MEDIUM, LOW).
    pub severity: String, // HIGH, MEDIUM, LOW
    /// Issue description.
    pub message: String,
    /// File where the issue is located.
    pub file: String,
    /// Line number of the issue.
    pub line: usize,
    /// Link to the file/line.
    pub link: String,
    /// Snippet of code surrounding the issue.
    pub code_snippet: Option<String>,
}

#[derive(Template)]
#[template(path = "file_view.html")]
/// View model for the file viewer page.
pub struct FileViewTemplate {
    /// CytoScnPy version.
    pub version: String,
    /// Relative path of the file.
    pub relative_path: String,
    /// File content (unused if highlight.js handles loading, but usually populated).
    pub code: String, // Using built-in highlight.js in client
    /// List of issues in this file.
    pub issues: Vec<IssueItem>,
    /// Source Lines of Code.
    pub sloc: usize,
    /// Cyclomatic Complexity.
    pub complexity: f64,
    /// Maintainability Index.
    pub mi: String,
    /// Raw Maintainability Index for logic.
    pub raw_mi: f64,
    /// Generation timestamp.
    pub generated_at: String,
    /// Root path for navigation (e.g. "." or "..")
    pub root_path: String,
}

/// View model for the Files list page.
#[derive(Template)]
#[template(path = "files.html")]
pub struct FilesTemplate {
    /// List of metrics for all files.
    pub file_metrics: Vec<FileMetricsView>,
    /// Average MI string for the project.
    pub average_mi: String,
    /// Color for the average MI.
    pub average_mi_color: String,
    /// CytoScnPy version.
    pub version: String,
    /// Generation timestamp.
    pub generated_at: String,
    /// Root path for navigation (e.g. "." or "..")
    pub root_path: String,
}
