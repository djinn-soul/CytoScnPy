//! Request schemas exposed by the CytoScnPy MCP tools.

use schemars::JsonSchema;

/// Request parameters for `analyze_path` tool.
#[derive(Debug, serde::Deserialize, JsonSchema)]
#[allow(clippy::struct_excessive_bools)]
pub struct AnalyzePathRequest {
    /// Path to the Python file or directory to analyze.
    #[schemars(description = "Path to the Python file or directory to analyze")]
    pub path: String,
    /// Whether to scan for hardcoded secrets (default: true).
    #[schemars(description = "Whether to scan for hardcoded secrets")]
    #[serde(default = "default_true")]
    pub scan_secrets: bool,
    /// Whether to scan for dangerous code patterns (default: true).
    #[schemars(description = "Whether to scan for dangerous code patterns like eval/exec")]
    #[serde(default = "default_true")]
    pub scan_danger: bool,
    /// Whether to check code quality metrics (default: true).
    #[schemars(description = "Whether to check code quality metrics")]
    #[serde(default = "default_true")]
    pub check_quality: bool,
}

const fn default_true() -> bool {
    true
}

/// Request parameters for `analyze_code` tool.
#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct AnalyzeCodeRequest {
    /// The Python code to analyze.
    #[schemars(description = "The Python code to analyze")]
    pub code: String,
    /// Virtual filename for the code (default: "snippet.py").
    #[schemars(description = "Virtual filename for the code snippet")]
    #[serde(default = "default_filename")]
    pub filename: String,
}

fn default_filename() -> String {
    "snippet.py".to_owned()
}

/// Request parameters for metrics tools.
#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct MetricsRequest {
    /// Path to the Python file or directory to analyze.
    #[schemars(description = "Path to the Python file or directory to analyze")]
    pub path: String,
}
