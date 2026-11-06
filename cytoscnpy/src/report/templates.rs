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
    /// CSS color for the grade (a theme variable such as `var(--grade-a)`).
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
    /// CSS color for the volume level (a theme variable).
    pub volume_color: String,
    /// Icon for volume level.
    pub volume_icon: String,
    /// Difficulty level to read/maintain.
    pub difficulty: String,
    /// Qualitative level for difficulty.
    pub difficulty_level: String,
    /// CSS color for the difficulty level (a theme variable).
    pub difficulty_color: String,
    /// Icon for difficulty level.
    pub difficulty_icon: String,
    /// Estimated effort to implement.
    pub effort: String,
    /// Qualitative level for effort.
    pub effort_level: String,
    /// CSS color for the effort level (a theme variable).
    pub effort_color: String,
    /// Icon for effort level.
    pub effort_icon: String,
    /// Estimated number of delivered bugs.
    pub bugs: String,
    /// Qualitative level for bugs.
    pub bugs_level: String,
    /// CSS color for the bugs level (a theme variable).
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
    /// CSS color for the score (a theme variable).
    pub score_color: String,
    /// Total number of files analyzed.
    pub total_files: usize,
    /// Total lines of code.
    pub total_lines: usize,
    /// Total issues found.
    pub total_issues: usize,
    /// CSS color for the total issues count (a theme variable).
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
    /// CSS color for the average MI (a theme variable).
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
    pub security: Vec<IssueItem>,
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
    /// Report tab the issue belongs to: `Unused`, `Security`, or anything
    /// else for the Code Quality tab.
    pub category: String,
    /// Finer-grained group shown as a chip and used by the category filter
    /// (e.g. `Injection`, `Secrets`, `Unused import`).
    pub group: String,
    /// Normalized severity: `CRITICAL`, `HIGH`, `MEDIUM`, or `LOW`.
    pub severity: String,
    /// Rule that produced the issue; empty when the issue has no rule.
    pub rule_id: String,
    /// Identifier matching the `stable_id` of the JSON report.
    pub stable_id: String,
    /// Issue description.
    pub message: String,
    /// File where the issue is located.
    pub file: String,
    /// Line number of the issue, relative to its notebook cell when `cell` is set.
    pub line: usize,
    /// Notebook cell index, for issues in a notebook.
    pub cell: Option<usize>,
    /// Line of the issue in the report's file view (cells are joined there).
    pub view_line: usize,
    /// Link to the file/line.
    pub link: String,
    /// Detection confidence (0-100), when the finding has one.
    pub confidence: Option<u8>,
    /// Extra labelled facts such as entropy or remediation advice.
    pub details: Vec<IssueDetail>,
    /// Taint flow from source to sink, empty for other issues.
    pub flow: Vec<String>,
    /// Whether the analyzer attached an automatic fix.
    pub fixable: bool,
    /// Comment that suppresses this issue, e.g. `# noqa: CSP-D003`; empty
    /// when the issue cannot be suppressed inline.
    pub noqa: String,
    /// Snippet of code surrounding the issue.
    pub code_snippet: Option<String>,
}

impl IssueItem {
    /// Human-readable location, including the notebook cell when known.
    #[must_use]
    pub fn location(&self) -> String {
        match self.cell {
            Some(cell) => format!("{} · cell {cell}, line {}", self.file, self.line),
            None => format!("{}:{}", self.file, self.line),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
/// A labelled fact shown under an issue.
pub struct IssueDetail {
    /// Label, e.g. `Entropy`.
    pub label: String,
    /// Value, e.g. `4.81`.
    pub value: String,
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
    /// Notebook cells as `(cell index, first line in the view)`; empty for
    /// Python files.
    pub cells: Vec<(usize, usize)>,
}

/// View model for the Files list page.
#[derive(Template)]
#[template(path = "files.html")]
pub struct FilesTemplate {
    /// List of metrics for all files.
    pub file_metrics: Vec<FileMetricsView>,
    /// Average MI string for the project.
    pub average_mi: String,
    /// CSS color for the average MI (a theme variable).
    pub average_mi_color: String,
    /// CytoScnPy version.
    pub version: String,
    /// Generation timestamp.
    pub generated_at: String,
    /// Root path for navigation (e.g. "." or "..")
    pub root_path: String,
}

#[derive(Debug, Clone, Serialize)]
/// Represents a clone finding to be displayed in the report.
pub struct CloneItem {
    /// Similarity percentage (0-100).
    pub similarity: f64,
    /// Clone type (Type-1, Type-2, Type-3).
    pub clone_type: String,
    /// Name of the function/class/block.
    pub name: String,
    /// File where the clone is located.
    pub file: String,
    /// Line number of the clone.
    pub line: usize,
    /// Link to the file/line.
    pub link: String,
    /// Related file (original or other copy).
    pub related_file: String,
    /// Line number in related file.
    pub related_line: usize,
    /// Link to related file/line.
    pub related_link: String,
    /// Rule that reported the clone, e.g. `CSP-C001`.
    pub rule_id: String,
    /// Comment that suppresses this clone finding on its line.
    pub noqa: String,
}

/// View model for the Clones report page.
#[derive(Template)]
#[template(path = "clones.html")]
pub struct ClonesTemplate {
    /// List of clone findings.
    pub clones: Vec<CloneItem>,
    /// Generation timestamp.
    pub generated_at: String,
    /// CytoScnPy version.
    pub version: String,
    /// Root path for navigation (e.g. "." or "..")
    pub root_path: String,
}

/// One scored `DeSlopify` dimension, formatted for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeslopDimensionView {
    /// Dimension name, e.g. `Architecture clarity`.
    pub name: String,
    /// Weight of the dimension (all weights sum to 100).
    pub weight: u32,
    /// Slop rating from 0 (pristine) to 5 (maximum slop).
    pub rating: u32,
    /// Weighted contribution to the raw score, formatted.
    pub contribution: String,
    /// Explanation of the rating.
    pub evidence: String,
}

/// One remediation recommendation, formatted for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeslopRecommendationView {
    /// Short title.
    pub title: String,
    /// Dimension the recommendation targets.
    pub dimension: String,
    /// Estimated Slop Index reduction in points.
    pub estimated_reduction: u32,
    /// Estimated effort: `Low`, `Medium`, or `High`.
    pub effort: String,
    /// Problem explanation.
    pub description: String,
    /// Steps to address it.
    pub action_steps: Vec<String>,
    /// Files directly affected.
    pub affected_files: Vec<String>,
}

/// A failed `DeSlopify` gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeslopGateView {
    /// Gate name, e.g. `max_slop_index`.
    pub check: String,
    /// Measured value.
    pub actual: String,
    /// Configured limit.
    pub limit: String,
}

/// The `DeSlopify` result shown on the report's Deslop page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeslopView {
    /// Final Slop Index, 0 (clean) to 100 (disaster).
    pub slop_index: u32,
    /// Verdict band, e.g. `ACCEPTABLE`.
    pub verdict: String,
    /// Raw score before the size multiplier, formatted.
    pub raw_score: String,
    /// Size multiplier, formatted.
    pub size_multiplier: String,
    /// Whether every gate passed.
    pub passed_gate: bool,
    /// Scored dimensions.
    pub dimensions: Vec<DeslopDimensionView>,
    /// Remediation recommendations, highest impact first.
    pub recommendations: Vec<DeslopRecommendationView>,
    /// Gates that failed.
    pub gate_failures: Vec<DeslopGateView>,
    /// Python files discovered by the scan.
    pub files_discovered: usize,
    /// Python files the scan could check.
    pub files_checked: usize,
    /// Files skipped as `(path, reason)`.
    pub skipped_files: Vec<(String, String)>,
}

impl DeslopView {
    /// CSS color for the verdict, from clean (green) to disaster (red).
    #[must_use]
    pub fn verdict_color(&self) -> &'static str {
        match self.verdict.as_str() {
            "CLEAN" => "var(--grade-a)",
            "ACCEPTABLE" => "var(--grade-b)",
            "MESSY" => "var(--grade-c)",
            "SLOPPY" => "var(--grade-d)",
            _ => "var(--grade-f)",
        }
    }
}

/// View model for the Deslop page.
#[derive(Template)]
#[template(path = "deslop.html")]
pub struct DeslopTemplate {
    /// The `DeSlopify` result, or `None` for the page shown before
    /// `cytoscnpy deslop --html` has run.
    pub deslop: Option<DeslopView>,
    /// True when the page is written without the rest of the report, so the
    /// navigation shows only this page.
    pub standalone: bool,
    /// Generation timestamp.
    pub generated_at: String,
    /// CytoScnPy version.
    pub version: String,
    /// Root path for navigation (e.g. "." or "..")
    pub root_path: String,
    /// Theme this page opens in, overriding the report's default.
    pub theme: Option<&'static str>,
}
