//! Core types for taint analysis.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Severity levels for taint findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// Critical severity - immediate exploitation risk
    Critical,
    /// High severity - significant security risk
    High,
    /// Medium severity - potential security risk
    Medium,
    /// Low severity - minor security concern
    Low,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Critical => write!(f, "CRITICAL"),
            Severity::High => write!(f, "HIGH"),
            Severity::Medium => write!(f, "MEDIUM"),
            Severity::Low => write!(f, "LOW"),
        }
    }
}

/// Vulnerability types detected by taint analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VulnType {
    /// SQL Injection
    SqlInjection,
    /// Command/OS Injection
    CommandInjection,
    /// Code Injection (eval/exec)
    CodeInjection,
    /// Path Traversal
    PathTraversal,
    /// Server-Side Request Forgery
    Ssrf,
    /// Cross-Site Scripting
    Xss,
    /// Insecure Deserialization
    Deserialization,
    /// Open Redirect
    OpenRedirect,
}

impl std::fmt::Display for VulnType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VulnType::SqlInjection => write!(f, "SQL Injection"),
            VulnType::CommandInjection => write!(f, "Command Injection"),
            VulnType::CodeInjection => write!(f, "Code Injection"),
            VulnType::PathTraversal => write!(f, "Path Traversal"),
            VulnType::Ssrf => write!(f, "SSRF"),
            VulnType::Xss => write!(f, "XSS"),
            VulnType::Deserialization => write!(f, "Insecure Deserialization"),
            VulnType::OpenRedirect => write!(f, "Open Redirect"),
        }
    }
}

/// Represents the origin of tainted data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaintSource {
    /// Flask request object (args, form, data, json, cookies, files)
    FlaskRequest(String),
    /// Django request object (GET, POST, body, COOKIES)
    DjangoRequest(String),
    /// `FastAPI` parameter (Query, Path, Body, Form)
    FastApiParam(String),
    /// Azure Functions request object (params, `get_json`, `get_body`, `route_params`)
    AzureFunctionsRequest(String),
    /// Python `input()` builtin
    Input,
    /// Environment variable (os.environ, os.getenv)
    Environment,
    /// Command line arguments (sys.argv)
    CommandLine,
    /// File read operation
    FileRead,
    /// External data (JSON/YAML load)
    ExternalData,
    /// Function parameter (for interprocedural)
    FunctionParam(String),
    /// Return value from tainted function
    FunctionReturn(String),
}

impl std::fmt::Display for TaintSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaintSource::FlaskRequest(attr) => write!(f, "Flask request.{attr}"),
            TaintSource::DjangoRequest(attr) => write!(f, "Django request.{attr}"),
            TaintSource::FastApiParam(name) => write!(f, "FastAPI param: {name}"),
            TaintSource::AzureFunctionsRequest(attr) => write!(f, "Azure Functions request.{attr}"),
            TaintSource::Input => write!(f, "input()"),
            TaintSource::Environment => write!(f, "environment variable"),
            TaintSource::CommandLine => write!(f, "sys.argv"),
            TaintSource::FileRead => write!(f, "file read"),
            TaintSource::ExternalData => write!(f, "external data"),
            TaintSource::FunctionParam(name) => write!(f, "function param: {name}"),
            TaintSource::FunctionReturn(name) => write!(f, "return from {name}"),
        }
    }
}

/// Information about a tainted variable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaintInfo {
    /// Original source of the taint
    pub source: TaintSource,
    /// Line where the variable was first tainted
    pub source_line: usize,
    /// Propagation path (variable names)
    pub path: Vec<String>,
}

impl TaintInfo {
    /// Creates a new taint info from a source.
    pub fn new(source: TaintSource, line: usize) -> Self {
        Self {
            source,
            source_line: line,
            path: Vec::new(),
        }
    }

    /// Extends the taint path with a new variable.
    #[must_use]
    pub fn extend_path(&self, var_name: &str) -> Self {
        let mut new_path = self.path.clone();
        new_path.push(var_name.to_owned());
        Self {
            source: self.source.clone(),
            source_line: self.source_line,
            path: new_path,
        }
    }
}

/// A finding from taint analysis - tainted data reaching a dangerous sink.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaintFinding {
    /// Source of the taint
    pub source: String,
    /// Line where taint originated
    pub source_line: usize,
    /// Sink function/pattern
    pub sink: String,
    /// Line where sink is called
    pub sink_line: usize,
    /// Column of sink
    pub sink_col: usize,
    /// Data flow path from source to sink
    pub flow_path: Vec<String>,
    /// Type of vulnerability
    pub vuln_type: VulnType,
    /// Severity level
    pub severity: Severity,
    /// File where finding was detected
    pub file: PathBuf,
    /// Suggested remediation
    pub remediation: String,
}

impl TaintFinding {
    /// Creates a formatted flow path string.
    pub fn flow_path_str(&self) -> String {
        if self.flow_path.is_empty() {
            format!("{} → {}", self.source, self.sink)
        } else {
            let path = self.flow_path.join(" → ");
            format!("{} → {} → {}", self.source, path, self.sink)
        }
    }
}

/// Function taint summary for interprocedural analysis.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FunctionSummary {
    /// Function name
    pub name: String,
    /// Which parameters propagate taint to return value (index -> true)
    pub param_to_return: Vec<bool>,
    /// Which parameters flow to which sinks
    pub param_to_sinks: Vec<(usize, VulnType)>,
    /// Does the function return tainted data from internal sources
    pub returns_tainted: bool,
    /// Does the function contain sinks
    pub has_sinks: bool,
}

impl FunctionSummary {
    /// Creates an empty summary for a function.
    pub fn new(name: &str, param_count: usize) -> Self {
        Self {
            name: name.to_owned(),
            param_to_return: vec![false; param_count],
            param_to_sinks: Vec::new(),
            returns_tainted: false,
            has_sinks: false,
        }
    }
}
