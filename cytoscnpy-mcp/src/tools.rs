//! MCP Tool implementations for CytoScnPy.
//!
//! This module defines the tools that are exposed via MCP, allowing LLMs
//! to perform Python static analysis using CytoScnPy.

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::commands::{run_cc, run_mi};
use rmcp::{
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_router, ErrorData as McpError, ServerHandler,
};
use schemars::JsonSchema;
use std::path::PathBuf;

/// Request parameters for analyze_path tool.
#[derive(Debug, serde::Deserialize, JsonSchema)]
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
    /// Whether to run taint analysis (default: false).
    #[schemars(description = "Whether to run taint/data-flow analysis")]
    #[serde(default)]
    pub taint_analysis: bool,
}

fn default_true() -> bool {
    true
}

/// Request parameters for analyze_code tool.
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

/// The main MCP server struct for CytoScnPy.
#[derive(Debug, Clone)]
pub struct CytoScnPyServer {
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl CytoScnPyServer {
    /// Creates a new CytoScnPy MCP server instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for CytoScnPyServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl CytoScnPyServer {
    /// Analyze Python code at the specified path for unused code, secrets, and quality issues.
    #[tool(
        description = "Analyze Python code at a path for unused code, secrets, dangerous patterns, and quality issues. Returns JSON with findings."
    )]
    pub fn analyze_path(
        &self,
        params: Parameters<AnalyzePathRequest>,
    ) -> Result<CallToolResult, McpError> {
        let req = params.0;
        let path_buf = PathBuf::from(&req.path);

        if !path_buf.exists() {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Path does not exist: {}",
                req.path
            ))]));
        }

        let mut analyzer = CytoScnPy::default()
            .with_secrets(req.scan_secrets)
            .with_danger(req.scan_danger)
            .with_quality(req.check_quality)
            .with_taint(req.taint_analysis);

        match analyzer.analyze(path_buf.as_path()) {
            Ok(result) => {
                let json = serde_json::to_string_pretty(&result)
                    .unwrap_or_else(|e| format!(r#"{{"error": "Serialization error: {}"}}"#, e));
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Analysis error: {}",
                e
            ))])),
        }
    }

    /// Analyze a Python code snippet directly without needing a file.
    #[tool(
        description = "Analyze a Python code snippet directly for unused code, secrets, and issues. Useful for code not saved to disk."
    )]
    pub fn analyze_code(
        &self,
        params: Parameters<AnalyzeCodeRequest>,
    ) -> Result<CallToolResult, McpError> {
        let req = params.0;
        let analyzer = CytoScnPy::default()
            .with_secrets(true)
            .with_danger(true)
            .with_quality(true);

        let result = analyzer.analyze_code(&req.code, PathBuf::from(&req.filename));

        let json = serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!(r#"{{"error": "Serialization error: {}"}}"#, e));
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Calculate cyclomatic complexity for Python code.
    #[tool(
        description = "Calculate cyclomatic complexity for Python code. Returns complexity scores with A-F ranking for each function."
    )]
    fn cyclomatic_complexity(
        &self,
        params: Parameters<MetricsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let req = params.0;
        let path_buf = PathBuf::from(&req.path);

        if !path_buf.exists() {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Path does not exist: {}",
                req.path
            ))]));
        }

        let mut output = Vec::new();
        match run_cc(
            path_buf,
            true, // JSON output
            vec![],
            vec![],
            None,
            None,
            false,
            false,
            true,
            None,
            false,
            false,
            None,
            None,
            &mut output,
        ) {
            Ok(()) => {
                let text = String::from_utf8(output)
                    .unwrap_or_else(|e| format!(r#"{{"error": "UTF-8 error: {}"}}"#, e));
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Analysis error: {}",
                e
            ))])),
        }
    }

    /// Calculate Maintainability Index for Python code.
    #[tool(
        description = "Calculate Maintainability Index (0-100) for Python code. Higher scores indicate better maintainability."
    )]
    fn maintainability_index(
        &self,
        params: Parameters<MetricsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let req = params.0;
        let path_buf = PathBuf::from(&req.path);

        if !path_buf.exists() {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Path does not exist: {}",
                req.path
            ))]));
        }

        let mut output = Vec::new();
        match run_mi(
            path_buf,
            true, // JSON output
            vec![],
            vec![],
            None,
            None,
            false,
            true, // show details
            false,
            None,
            None,
            &mut output,
        ) {
            Ok(()) => {
                let text = String::from_utf8(output)
                    .unwrap_or_else(|e| format!(r#"{{"error": "UTF-8 error: {}"}}"#, e));
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Analysis error: {}",
                e
            ))])),
        }
    }
}

#[rmcp::tool_handler]
impl ServerHandler for CytoScnPyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "CytoScnPy is a high-performance Python static analyzer. \
                 Use it to find unused code, detect secrets, identify dangerous patterns, \
                 and measure code quality metrics like cyclomatic complexity and maintainability index."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
