//! Tests for output formatting and serialization.
#![allow(clippy::default_trait_access)]
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::{AnalysisResult, AnalysisSummary};
use cytoscnpy::rules::Finding;
use cytoscnpy::visitor::Definition;
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn test_print_report_formatting() {
    colored::control::set_override(false);
    // Create a mock AnalysisResult
    let result = AnalysisResult {
        unused_functions: vec![Definition {
            name: "unused_func".to_owned(),
            full_name: "module.unused_func".to_owned(),
            simple_name: "unused_func".to_owned(),
            def_type: "function".to_owned(),
            file: Arc::new(PathBuf::from("test.py")),
            line: 10,
            end_line: 10,
            col: 0,
            start_byte: 0,
            end_byte: 0,
            confidence: 100,
            references: 0,
            is_exported: false,
            in_init: false,
            is_framework_managed: false,
            base_classes: smallvec::smallvec![],
            is_type_checking: false,
            is_captured: false,
            cell_number: None,
            is_self_referential: false,
            message: Some("'unused_func' is defined but never used".to_owned()),
            fix: None,
            is_enum_member: false,
            is_constant: false,
            is_potential_secret: false,
            is_unreachable: false,
            category: cytoscnpy::visitor::UnusedCategory::default(),
        }],
        unused_methods: vec![],
        unused_imports: vec![],
        unused_classes: vec![],
        unused_variables: vec![],
        unused_parameters: vec![],
        secrets: vec![],
        danger: vec![Finding {
            message: "Dangerous eval".to_owned(),
            rule_id: "CSP-D001".to_owned(),
            category: "Code Execution".to_owned(),
            file: PathBuf::from("danger.py"),
            line: 5,
            col: 0,
            severity: "CRITICAL".to_owned(),
        }],
        quality: vec![],
        taint_findings: vec![],
        parse_errors: vec![],
        clones: vec![],
        file_metrics: vec![],
        analysis_summary: AnalysisSummary {
            total_files: 5,
            secrets_count: 0,
            danger_count: 1,
            quality_count: 0,
            taint_count: 0,
            parse_errors_count: 0,
            total_lines_analyzed: 100,
            total_definitions: 0,
            average_complexity: 0.0,
            average_mi: 0.0,
            total_directories: 0,
            total_size: 0.0,
            functions_count: 0,
            classes_count: 0,
            raw_metrics: Default::default(),
            halstead_metrics: Default::default(),
        },
    };

    // Capture output in a buffer
    let mut buffer = Vec::new();
    cytoscnpy::output::print_report(&mut buffer, &result).unwrap();
    cytoscnpy::output::print_summary_pills(&mut buffer, &result).unwrap();
    cytoscnpy::output::print_analysis_stats(&mut buffer, &result.analysis_summary).unwrap();

    // Convert buffer to string (ignoring color codes for simple assertions)
    let output = String::from_utf8_lossy(&buffer);

    // Assertions
    assert!(output.contains("Python Static Analysis Results"));
    assert!(output.contains("Analyzed 5 files (100 lines)"));

    // Check for unused function
    assert!(output.contains("Unreachable Functions"));
    assert!(output.contains("unused_func"));
    assert!(output.contains("test.py:10"));

    // Check for security issue
    assert!(output.contains("Security Issues"));
    assert!(output.contains("Dangerous eval"));
    assert!(output.contains("CSP-D001"));
    assert!(output.contains("CRITICAL"));
    assert!(output.contains("danger.py:5"));

    // Check for box drawing characters from comfy-table (UTF8_FULL preset)
    assert!(output.contains("┌"));
    assert!(output.contains("┐"));
    assert!(output.contains("│"));
    assert!(output.contains("└"));
    assert!(output.contains("┘"));

    // Check for table headers
    assert!(output.contains("Rule ID"));
    assert!(output.contains("Message"));
    assert!(output.contains("Location"));
    assert!(output.contains("Severity"));
    assert!(output.contains("Type"));
    assert!(output.contains("Name"));
}
