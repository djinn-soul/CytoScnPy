//! Tests for the various report formats.
//!
//! Checks that reports (JSON, SARIF, `GitHub`, etc.) are generated correctly.

#![allow(clippy::unwrap_used)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::manual_string_new)]
#![allow(clippy::clone_on_ref_ptr)]
#![allow(clippy::useless_vec)]

use cytoscnpy::analyzer::types::{AnalysisResult, AnalysisSummary, FileMetrics, ParseError};
use cytoscnpy::commands::run_init_in;
use cytoscnpy::report::{github, gitlab, junit, markdown, sarif};
use cytoscnpy::rules::secrets::SecretFinding;
use cytoscnpy::rules::Finding;
use cytoscnpy::taint::types::{Severity as TaintSeverity, TaintFinding, VulnType};
use cytoscnpy::visitor::Definition;
use smallvec::smallvec;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::tempdir;

fn create_mock_result() -> AnalysisResult {
    let file = PathBuf::from("test.py");
    let arc_file = Arc::new(file.clone());

    let mut result = AnalysisResult {
        unused_functions: vec![],
        unused_methods: vec![],
        unused_imports: vec![],
        unused_classes: vec![],
        unused_variables: vec![],
        unused_parameters: vec![],
        secrets: vec![],
        danger: vec![],
        quality: vec![],
        taint_findings: vec![],
        parse_errors: vec![],
        clones: vec![],
        file_metrics: vec![],
        analysis_summary: AnalysisSummary::default(),
    };

    result.danger.push(Finding {
        rule_id: "CSP-D001".to_owned(),
        category: "Security".to_owned(),
        severity: "HIGH".to_owned(),
        message: "Dangerous function call".to_owned(),
        file: file.clone(),
        line: 10,
        col: 5,
    });

    result.secrets.push(SecretFinding {
        message: "Hardcoded password".to_owned(),
        rule_id: "CSP-S001".to_owned(),
        category: "Secrets".to_owned(),
        file: file.clone(),
        line: 20,
        severity: "CRITICAL".to_owned(),
        matched_value: None,
        entropy: None,
        confidence: 100,
    });

    result.quality.push(Finding {
        rule_id: "CSP-Q001".to_owned(),
        category: "Maintainability".to_owned(),
        severity: "MEDIUM".to_owned(),
        message: "High complexity".to_owned(),
        file: file.clone(),
        line: 5,
        col: 0,
    });

    result.taint_findings.push(TaintFinding {
        rule_id: "CSP-T001".to_owned(),
        vuln_type: VulnType::SqlInjection,
        severity: TaintSeverity::High,
        file: file.clone(),
        source: "request.args".to_owned(),
        sink: "execute".to_owned(),
        sink_line: 15,
        sink_col: 10,
        source_line: 12,
        flow_path: vec![],
        category: "Taint Analysis".to_owned(),
        remediation: "Use parameterized queries".to_owned(),
    });

    let def_base = Definition {
        name: "".to_owned(),
        full_name: "".to_owned(),
        simple_name: "".to_owned(),
        def_type: "".to_owned(),
        file: arc_file.clone(),
        line: 30,
        end_line: 35,
        col: 0,
        start_byte: 0,
        end_byte: 100,
        confidence: 100,
        references: 0,
        is_exported: false,
        in_init: false,
        is_framework_managed: false,
        base_classes: smallvec![],
        is_type_checking: false,
        is_captured: false,
        cell_number: None,
        is_self_referential: false,
        message: None,
        fix: None,
        is_enum_member: false,
        is_constant: false,
        is_potential_secret: false,
        is_unreachable: false,
        category: cytoscnpy::visitor::UnusedCategory::default(),
    };

    result.unused_functions.push(Definition {
        name: "f".into(),
        full_name: "f".into(),
        simple_name: "f".into(),
        def_type: "function".into(),
        message: Some("unused func".into()),
        ..def_base.clone()
    });
    result.unused_classes.push(Definition {
        name: "C".into(),
        full_name: "C".into(),
        simple_name: "C".into(),
        def_type: "class".into(),
        message: Some("unused class".into()),
        ..def_base.clone()
    });
    result.unused_methods.push(Definition {
        name: "m".into(),
        full_name: "m".into(),
        simple_name: "m".into(),
        def_type: "method".into(),
        message: Some("unused method".into()),
        ..def_base.clone()
    });
    result.unused_imports.push(Definition {
        name: "i".into(),
        full_name: "i".into(),
        simple_name: "i".into(),
        def_type: "import".into(),
        message: Some("unused import".into()),
        ..def_base.clone()
    });
    result.unused_variables.push(Definition {
        name: "v".into(),
        full_name: "v".into(),
        simple_name: "v".into(),
        def_type: "variable".into(),
        message: Some("unused var".into()),
        ..def_base.clone()
    });
    result.unused_parameters.push(Definition {
        name: "p".into(),
        full_name: "p".into(),
        simple_name: "p".into(),
        def_type: "parameter".into(),
        message: Some("unused param".into()),
        ..def_base
    });

    result.parse_errors.push(ParseError {
        file: file.clone(),
        error: "Syntax error at line 40".to_owned(),
    });

    result.file_metrics.push(FileMetrics {
        file: file.clone(),
        loc: 100,
        sloc: 80,
        complexity: 5.0,
        mi: 70.0,
        total_issues: 5,
    });

    result.analysis_summary = AnalysisSummary {
        total_files: 1,
        danger_count: 1,
        secrets_count: 1,
        quality_count: 1,
        taint_count: 1,
        parse_errors_count: 1,
        total_lines_analyzed: 100,
        ..AnalysisSummary::default()
    };

    result
}

#[test]
fn test_github_report_coverage() {
    let result = create_mock_result();
    let mut buffer = Vec::new();
    github::print_github(&mut buffer, &result).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    assert!(output
        .contains("::error file=test.py,line=10,col=5,title=CSP-D001::Dangerous function call (test.py:10)"));
    assert!(output.contains("UnusedMethod"));
    assert!(output.contains("UnusedClass"));
    assert!(output.contains("UnusedImport"));
    assert!(output.contains("UnusedVariable"));
    assert!(output.contains("UnusedParameter"));
}

#[test]
fn test_junit_report_coverage() {
    let result = create_mock_result();
    let mut buffer = Vec::new();
    junit::print_junit(&mut buffer, &result).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    // mock_result has: 1 danger + 1 secret + 1 quality + 1 taint + 1 parse_error + 6 unused = 11 findings
    assert!(output.contains(r#"<testsuite name="CytoScnPy" tests="11" failures="11" errors="0">"#));
    // The current implementation does not include the 'type' attribute in the failure tag
    assert!(output.contains(r#"<failure message="Dangerous function call">Line 10: Dangerous function call (test.py:10)</failure>"#));
}

#[test]
fn test_markdown_report_coverage() {
    let result = create_mock_result();
    let mut buffer = Vec::new();
    markdown::print_markdown(&mut buffer, &result).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("# CytoScnPy Analysis Report"));
    assert!(output.contains("## Security Issues"));
    // Matches: | Rule | File | Line | Message | Severity |
    // | CSP-D001 | test.py | 10 | Dangerous function call | HIGH |
    assert!(output.contains("| CSP-D001 | test.py | 10 | Dangerous function call | HIGH |"));
}

#[test]
fn test_gitlab_report_coverage() {
    let result = create_mock_result();
    let mut buffer = Vec::new();
    gitlab::print_gitlab(&mut buffer, &result).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains(r#""description": "Dangerous function call (test.py:10)""#));
    // Implementation uses: danger-{rule_id}-{normalized_path}-{index}
    // Result has rule_id="CSP-D001", file="test.py", index=0 (first danger finding)
    // "fingerprint": "danger-CSP-D001-test.py-0"
    assert!(output.contains(r#""fingerprint": "danger-CSP-D001-test.py-0""#));
}

#[test]
fn test_sarif_report_coverage() {
    let result = create_mock_result();
    let mut buffer = Vec::new();
    sarif::print_sarif(&mut buffer, &result).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains(r#""version": "2.1.0""#));
    // SARIF uses "ruleId" for results
    assert!(output.contains(r#""ruleId": "CSP-D001""#));
    // Check path in message
    assert!(output.contains(r#""text": "Dangerous function call (test.py:10)""#));
}

#[test]
fn test_init_command_full_coverage() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let mut buffer = Vec::new();

    // 1. Init in empty dir -> creates cytoscnpy.toml
    run_init_in(root, &mut buffer).unwrap();
    assert!(root.join(".cytoscnpy.toml").exists());

    // 2. Init again -> skips
    buffer.clear();
    run_init_in(root, &mut buffer).unwrap();
    let output = String::from_utf8(buffer.clone()).unwrap();
    assert!(output.contains("already exists"));

    // 3. Init with existing pyproject.toml -> appends
    let dir2 = tempdir().unwrap();
    let root2 = dir2.path();
    std::fs::write(root2.join("pyproject.toml"), "[project]\nname = 'test'").unwrap();
    buffer.clear();
    run_init_in(root2, &mut buffer).unwrap();
    let content = std::fs::read_to_string(root2.join("pyproject.toml")).unwrap();
    assert!(content.contains("[tool.cytoscnpy]"));
}

#[test]
fn test_output_formatting_coverage() {
    use cytoscnpy::output;
    let result = create_mock_result();
    let mut buffer = Vec::new();

    // Disable colors for consistent string matching across all output tests
    colored::control::set_override(false);

    // Test header
    output::print_header(&mut buffer).unwrap();
    assert!(String::from_utf8(buffer.clone())
        .unwrap()
        .contains("Python Static Analysis Results"));

    // Test summary pills
    buffer.clear();
    output::print_summary_pills(&mut buffer, &result).unwrap();
    let out = String::from_utf8(buffer.clone()).unwrap();
    assert!(out.contains("Unreachable: 1"));

    // Test stats
    buffer.clear();
    output::print_analysis_stats(&mut buffer, &result.analysis_summary).unwrap();
    assert!(String::from_utf8(buffer.clone())
        .unwrap()
        .contains("Analyzed 1 files"));

    // Test findings
    buffer.clear();
    output::print_findings(&mut buffer, "Test Findings", &result.danger).unwrap();
    assert!(String::from_utf8(buffer.clone())
        .unwrap()
        .contains("CSP-D001"));

    // Test report grouped
    buffer.clear();
    output::print_report_grouped(&mut buffer, &result).unwrap();
    assert!(String::from_utf8(buffer.clone())
        .unwrap()
        .contains("File: test.py"));

    // Test report quiet
    buffer.clear();
    output::print_report_quiet(&mut buffer, &result).unwrap();
    assert!(String::from_utf8(buffer.clone())
        .unwrap()
        .contains("[SUMMARY]"));

    // Test exclusion list
    buffer.clear();
    output::print_exclusion_list(&mut buffer, &vec!["node_modules".to_owned()]).unwrap();
    assert!(String::from_utf8(buffer.clone())
        .unwrap()
        .contains("Excluding:"));

    // Test spinner and progress bar (smoke tests as they are hidden in test mode)
    let _pb = output::create_progress_bar(10);
    let _spinner = output::create_spinner();

    // Re-enable colors
    colored::control::unset_override();
}
