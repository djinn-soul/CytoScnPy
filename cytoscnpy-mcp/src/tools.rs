//! MCP Tool implementations for CytoScnPy.
//!
//! This module defines the tools that are exposed via MCP, allowing LLMs
//! to perform Python static analysis using CytoScnPy.

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::commands::{run_cc, run_mi};
use rmcp::{
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_router, ErrorData as McpError, ServerHandler,
};
use schemars::JsonSchema;
use std::path::PathBuf;

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

fn default_true() -> bool {
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
#[allow(clippy::unused_self, clippy::unnecessary_wraps)]
impl CytoScnPyServer {
    /// Analyze Python code at the specified path for unused code, secrets, and quality issues.
    ///
    /// # Errors
    ///
    /// Returns an error if the path does not exist or if analysis fails.
    #[tool(
        description = "Comprehensive Python static analysis for a file or directory. Finds: \n\
        • Unused functions, classes, imports, variables\n\
        • Hardcoded secrets (API keys, passwords)\n\
        • Dangerous patterns (eval, exec, pickle, shell injection)\n\
        • Code quality issues (complexity, mutable defaults)\n\
        Returns detailed JSON with severity levels (CRITICAL/HIGH/MEDIUM/LOW)."
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
            .with_quality(req.check_quality);

        let result = analyzer.analyze(path_buf.as_path());
        let json = serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!(r#"{{"error": "Serialization error: {e}"}}"#));
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Analyze a Python code snippet directly without needing a file.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization of the results fails.
    #[tool(
        description = "Analyze a Python code snippet directly without needing a file. \n\
        Perfect for: reviewing code from chat, analyzing pasted snippets, quick security checks.\n\
        Detects unused code, security issues, and quality problems. Returns JSON results."
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

        let result = analyzer.analyze_code(&req.code, &PathBuf::from(&req.filename));

        let json = serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!(r#"{{"error": "Serialization error: {e}"}}"#));
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Quick security scan - focuses only on secrets and dangerous patterns.
    ///
    /// # Errors
    ///
    /// Returns an error if the path does not exist or analysis fails.
    #[tool(description = "Fast security-focused scan. Only checks for:\\n\
        • Hardcoded secrets (API keys, passwords, tokens)\\n\
        • Dangerous patterns (eval, exec, pickle, shell injection, SSRF)\\n\
        Skips unused code detection for faster results. Perfect for CI/CD pipelines.")]
    pub fn quick_scan(
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

        // Security-only scan - no unused code detection
        let mut analyzer = CytoScnPy::default()
            .with_secrets(true)
            .with_danger(true)
            .with_quality(false);

        let result = analyzer.analyze(path_buf.as_path());

        // Return only security-relevant findings
        let security_summary = serde_json::json!({
            "scan_type": "quick_security_scan",
            "path": req.path,
            "summary": {
                "secrets_found": result.secrets.len(),
                "dangerous_patterns": result.danger.len(),
                "total_issues": result.secrets.len() + result.danger.len(),
            },
            "secrets": result.secrets,
            "danger": result.danger,
            "recommendation": if result.secrets.is_empty() && result.danger.is_empty() {
                "✅ No security issues found"
            } else {
                "⚠️ Security issues detected - review and fix immediately"
            }
        });

        let json = serde_json::to_string_pretty(&security_summary)
            .unwrap_or_else(|e| format!(r#"{{"error": "Serialization error: {e}"}}"#));
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Calculate cyclomatic complexity for Python code.
    ///
    /// # Errors
    ///
    /// Returns an error if the path doesn't exist or if analysis fails.
    #[tool(
        description = "Calculate McCabe cyclomatic complexity for Python functions.\n\
        Rankings: A (1-5 simple), B (6-10 moderate), C (11-20 complex), D (21-30 very complex), E/F (30+ unmaintainable).\n\
        High complexity indicates code that is hard to test and maintain. Aim for ≤10 per function."
    )]
    pub fn cyclomatic_complexity(
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
            &[path_buf],
            cytoscnpy::commands::CcOptions {
                json: true,
                output_file: None,
                ..Default::default()
            },
            &mut output,
        ) {
            Ok(()) => {
                let text = String::from_utf8(output)
                    .unwrap_or_else(|e| format!(r#"{{"error": "UTF-8 error: {e}"}}"#));
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Analysis error: {e}"
            ))])),
        }
    }

    /// Calculate Maintainability Index for Python code.
    ///
    /// # Errors
    ///
    /// Returns an error if the path doesn't exist or if analysis fails.
    #[tool(
        description = "Calculate Maintainability Index (MI) for Python files.\n\
        Scale: 0-100 where higher is better. Rankings: A (≥20 good), B (10-19 moderate), C (<10 poor).\n\
        MI combines Halstead volume, cyclomatic complexity, and lines of code. Aim for MI ≥ 40."
    )]
    pub fn maintainability_index(
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
            &[path_buf],
            cytoscnpy::commands::MiOptions {
                json: true,
                show: true,
                output_file: None,
                ..Default::default()
            },
            &mut output,
        ) {
            Ok(()) => {
                let text = String::from_utf8(output)
                    .unwrap_or_else(|e| format!(r#"{{"error": "UTF-8 error: {e}"}}"#));
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Analysis error: {e}"
            ))])),
        }
    }
}

#[rmcp::tool_handler]
impl ServerHandler for CytoScnPyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "CytoScnPy is a high-performance Python static analyzer built in Rust. \n\n\
                 🔍 TOOLS AVAILABLE:\n\
                 • analyze_path - Full analysis of files/directories\n\
                 • analyze_code - Analyze code snippets directly\n\
                 • quick_scan - Fast security-only check (secrets + dangerous patterns)\n\
                 • cyclomatic_complexity - Measure code complexity\n\
                 • maintainability_index - Measure maintainability\n\n\
                 📋 COMMON TASKS:\n\
                 • 'Quick security check' → quick_scan\n\
                 • 'Full analysis' → analyze_path with all flags\n\
                 • 'Find unused code' → analyze_path, check unused_functions/imports\n\
                 • 'Is this function too complex?' → cyclomatic_complexity\n\
                 • 'Rate code quality' → maintainability_index\n\n\
                 ⚠️ SEVERITY LEVELS: CRITICAL > HIGH > MEDIUM > LOW"
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

pub use rmcp::model::Content;
