//! Tests for taint analysis types module.
//! Increases coverage for `src/taint/types.rs`

#![allow(clippy::unwrap_used)]
#![allow(clippy::str_to_string)]

use cytoscnpy::taint::types::{
    FunctionSummary, Severity, TaintFinding, TaintInfo, TaintSource, VulnType,
};
use std::path::PathBuf;

// =============================================================================
// SEVERITY TESTS
// =============================================================================

#[test]
fn test_severity_display() {
    assert_eq!(format!("{}", Severity::Critical), "CRITICAL");
    assert_eq!(format!("{}", Severity::High), "HIGH");
    assert_eq!(format!("{}", Severity::Medium), "MEDIUM");
    assert_eq!(format!("{}", Severity::Low), "LOW");
}

#[test]
fn test_severity_equality() {
    assert_eq!(Severity::Critical, Severity::Critical);
    assert_ne!(Severity::High, Severity::Low);
}

// =============================================================================
// VULN TYPE TESTS
// =============================================================================

#[test]
fn test_vuln_type_display() {
    assert_eq!(format!("{}", VulnType::SqlInjection), "SQL Injection");
    assert_eq!(
        format!("{}", VulnType::CommandInjection),
        "Command Injection"
    );
    assert_eq!(format!("{}", VulnType::CodeInjection), "Code Injection");
    assert_eq!(format!("{}", VulnType::PathTraversal), "Path Traversal");
    assert_eq!(format!("{}", VulnType::Ssrf), "SSRF");
    assert_eq!(format!("{}", VulnType::Xss), "XSS");
    assert_eq!(
        format!("{}", VulnType::Deserialization),
        "Insecure Deserialization"
    );
    assert_eq!(format!("{}", VulnType::OpenRedirect), "Open Redirect");
}

#[test]
fn test_vuln_type_equality() {
    assert_eq!(VulnType::SqlInjection, VulnType::SqlInjection);
    assert_ne!(VulnType::SqlInjection, VulnType::CommandInjection);
}

// =============================================================================
// TAINT SOURCE TESTS
// =============================================================================

#[test]
fn test_taint_source_flask_display() {
    let source = TaintSource::FlaskRequest("args".to_string());
    assert_eq!(format!("{source}"), "Flask request.args");
}

#[test]
fn test_taint_source_django_display() {
    let source = TaintSource::DjangoRequest("GET".to_string());
    assert_eq!(format!("{source}"), "Django request.GET");
}

#[test]
fn test_taint_source_fastapi_display() {
    let source = TaintSource::FastApiParam("user_id".to_string());
    assert_eq!(format!("{source}"), "FastAPI param: user_id");
}

#[test]
fn test_taint_source_input_display() {
    assert_eq!(format!("{}", TaintSource::Input), "input()");
}

#[test]
fn test_taint_source_environment_display() {
    assert_eq!(
        format!("{}", TaintSource::Environment),
        "environment variable"
    );
}

#[test]
fn test_taint_source_command_line_display() {
    assert_eq!(format!("{}", TaintSource::CommandLine), "sys.argv");
}

#[test]
fn test_taint_source_file_read_display() {
    assert_eq!(format!("{}", TaintSource::FileRead), "file read");
}

#[test]
fn test_taint_source_external_data_display() {
    assert_eq!(format!("{}", TaintSource::ExternalData), "external data");
}

#[test]
fn test_taint_source_function_param_display() {
    let source = TaintSource::FunctionParam("data".to_string());
    assert_eq!(format!("{source}"), "function param: data");
}

#[test]
fn test_taint_source_function_return_display() {
    let source = TaintSource::FunctionReturn("get_data".to_string());
    assert_eq!(format!("{source}"), "return from get_data");
}

#[test]
fn test_taint_source_equality() {
    let source1 = TaintSource::FlaskRequest("args".to_string());
    let source2 = TaintSource::FlaskRequest("args".to_string());
    let source3 = TaintSource::FlaskRequest("form".to_string());

    assert_eq!(source1, source2);
    assert_ne!(source1, source3);
}

// =============================================================================
// TAINT INFO TESTS
// =============================================================================

#[test]
fn test_taint_info_new() {
    let info = TaintInfo::new(TaintSource::Input, 10);

    assert_eq!(info.source, TaintSource::Input);
    assert_eq!(info.source_line, 10);
    assert!(info.path.is_empty());
}

#[test]
fn test_taint_info_extend_path() {
    let info = TaintInfo::new(TaintSource::Input, 5);
    let extended = info.extend_path("user_data");

    assert_eq!(extended.source, TaintSource::Input);
    assert_eq!(extended.source_line, 5);
    assert_eq!(extended.path, vec!["user_data"]);
}

#[test]
fn test_taint_info_extend_path_multiple() {
    let info = TaintInfo::new(TaintSource::Input, 1);
    let extended1 = info.extend_path("raw_data");
    let extended2 = extended1.extend_path("processed");
    let extended3 = extended2.extend_path("final");

    assert_eq!(extended3.path.len(), 3);
    assert_eq!(extended3.path, vec!["raw_data", "processed", "final"]);
}

// =============================================================================
// TAINT FINDING TESTS
// =============================================================================

#[test]
fn test_taint_finding_flow_path_str_empty() {
    let finding = TaintFinding {
        source: "input()".to_string(),
        source_line: 1,
        sink: "eval()".to_string(),
        sink_line: 5,
        sink_col: 0,
        flow_path: vec![],
        rule_id: "CSP-D001".to_string(),
        vuln_type: VulnType::CodeInjection,
        category: "Taint Analysis".to_owned(),
        severity: Severity::Critical,
        file: PathBuf::from("test.py"),
        remediation: "Don't use eval".to_string(),
    };

    assert_eq!(finding.flow_path_str(), "input() → eval()");
}

#[test]
fn test_taint_finding_flow_path_str_with_path() {
    let finding = TaintFinding {
        source: "request.args".to_string(),
        source_line: 1,
        sink: "db.execute".to_string(),
        sink_line: 10,
        sink_col: 4,
        flow_path: vec!["user_input".to_string(), "query".to_string()],
        rule_id: "CSP-D102".to_string(),
        vuln_type: VulnType::SqlInjection,
        category: "Taint Analysis".to_owned(),
        severity: Severity::High,
        file: PathBuf::from("app.py"),
        remediation: "Use parameterized queries".to_string(),
    };

    assert_eq!(
        finding.flow_path_str(),
        "request.args → user_input → query → db.execute"
    );
}

// =============================================================================
// FUNCTION SUMMARY TESTS
// =============================================================================

#[test]
fn test_function_summary_new() {
    let summary = FunctionSummary::new("my_func", 3);

    assert_eq!(summary.name, "my_func");
    assert_eq!(summary.param_to_return.len(), 3);
    assert!(summary.param_to_return.iter().all(|&x| !x));
    assert!(summary.param_to_sinks.is_empty());
    assert!(!summary.returns_tainted);
    assert!(!summary.has_sinks);
}

#[test]
fn test_function_summary_zero_params() {
    let summary = FunctionSummary::new("no_args", 0);

    assert_eq!(summary.name, "no_args");
    assert!(summary.param_to_return.is_empty());
}

#[test]
fn test_function_summary_default() {
    let summary = FunctionSummary::default();

    assert!(summary.name.is_empty());
    assert!(summary.param_to_return.is_empty());
    assert!(!summary.returns_tainted);
}
