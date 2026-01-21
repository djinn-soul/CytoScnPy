//! Tests for output.rs - CLI output formatting functions.
#![allow(clippy::unwrap_used)]
#![allow(clippy::default_trait_access)]

use cytoscnpy::analyzer::{AnalysisResult, AnalysisSummary, ParseError};
use cytoscnpy::output::{
    create_progress_bar, create_spinner, print_analysis_stats, print_exclusion_list,
    print_findings, print_header, print_parse_errors, print_report, print_report_quiet,
    print_summary_pills, print_unused_items,
};
use cytoscnpy::rules::Finding;
use cytoscnpy::visitor::Definition;
use std::path::PathBuf;
use std::sync::Arc;

fn create_mock_result() -> AnalysisResult {
    AnalysisResult {
        unused_functions: vec![Definition {
            name: "unused_func".to_owned(),
            full_name: "test.unused_func".to_owned(),
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
            message: None,
            fix: None,
            is_enum_member: false,
            is_constant: false,
            is_potential_secret: false,
        }],
        unused_methods: vec![],
        unused_imports: vec![],
        unused_classes: vec![],
        unused_variables: vec![],
        unused_parameters: vec![],
        secrets: vec![],
        danger: vec![],
        quality: vec![Finding {
            message: "Test finding".to_owned(),
            rule_id: "CSP-Q001".to_owned(),
            category: "Maintainability".to_owned(),
            file: PathBuf::from("test.py"),
            line: 5,
            col: 0,
            severity: "MEDIUM".to_owned(),
        }],
        taint_findings: vec![],
        parse_errors: vec![],
        clones: vec![],
        file_metrics: vec![],
        analysis_summary: AnalysisSummary {
            total_files: 1,
            secrets_count: 0,
            danger_count: 0,
            quality_count: 1,
            taint_count: 0,
            parse_errors_count: 0,
            total_lines_analyzed: 100,
            total_definitions: 1,
            average_complexity: 5.0,
            average_mi: 70.0,
            total_directories: 0,
            total_size: 1.0,
            functions_count: 1,
            classes_count: 0,
            raw_metrics: Default::default(),
            halstead_metrics: Default::default(),
        },
    }
}

#[test]
fn test_print_exclusion_list_empty() {
    let mut buffer = Vec::new();
    let result = print_exclusion_list(&mut buffer, &[]);
    assert!(result.is_ok());
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("default exclusions"));
}

#[test]
fn test_print_exclusion_list_with_folders() {
    let mut buffer = Vec::new();
    let folders = vec!["build".to_owned(), "dist".to_owned()];
    let result = print_exclusion_list(&mut buffer, &folders);
    assert!(result.is_ok());
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("Excluding") || output.contains("build"));
}

#[test]
fn test_create_spinner() {
    let spinner = create_spinner();
    assert!(spinner.length().is_none()); // Spinner has no fixed length
    spinner.finish();
}

#[test]
fn test_create_progress_bar() {
    let pb = create_progress_bar(100);
    assert_eq!(pb.length(), Some(100));
    pb.finish();
}

#[test]
fn test_print_header() {
    let mut buffer = Vec::new();
    let result = print_header(&mut buffer);
    assert!(result.is_ok());
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("Python Static Analysis"));
}

#[test]
fn test_print_summary_pills() {
    let mut buffer = Vec::new();
    let result = create_mock_result();
    let res = print_summary_pills(&mut buffer, &result);
    assert!(res.is_ok());
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("Unreachable"));
}

#[test]
fn test_print_analysis_stats() {
    let mut buffer = Vec::new();
    let summary = AnalysisSummary {
        total_files: 10,
        total_lines_analyzed: 1000,
        average_complexity: 8.5,
        average_mi: 65.0,
        secrets_count: 0,
        danger_count: 0,
        quality_count: 0,
        taint_count: 0,
        parse_errors_count: 0,
        total_definitions: 0,
        total_directories: 0,
        total_size: 0.0,
        functions_count: 0,
        classes_count: 0,
        raw_metrics: cytoscnpy::raw_metrics::RawMetrics::default(),
        halstead_metrics: cytoscnpy::halstead::HalsteadMetrics::default(),
    };
    let result = print_analysis_stats(&mut buffer, &summary);
    assert!(result.is_ok());
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("10"));
    assert!(output.contains("1000"));
}

#[test]
fn test_print_analysis_stats_high_complexity() {
    let mut buffer = Vec::new();
    let summary = AnalysisSummary {
        total_files: 5,
        total_lines_analyzed: 500,
        average_complexity: 15.0, // High complexity
        average_mi: 30.0,         // Low MI
        secrets_count: 0,
        danger_count: 0,
        quality_count: 0,
        taint_count: 0,
        parse_errors_count: 0,
        total_definitions: 0,
        total_directories: 0,
        total_size: 0.0,
        functions_count: 0,
        classes_count: 0,
        raw_metrics: cytoscnpy::raw_metrics::RawMetrics::default(),
        halstead_metrics: cytoscnpy::halstead::HalsteadMetrics::default(),
    };
    let result = print_analysis_stats(&mut buffer, &summary);
    assert!(result.is_ok());
}

#[test]
fn test_print_findings_empty() {
    let mut buffer = Vec::new();
    let result = print_findings(&mut buffer, "Test", &[]);
    assert!(result.is_ok());
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.is_empty()); // Should print nothing for empty findings
}

#[test]
fn test_print_findings_with_items() {
    let mut buffer = Vec::new();
    let findings = vec![Finding {
        message: "Test message".to_owned(),
        rule_id: "TEST-001".to_owned(),
        category: "Security Issues".to_owned(),
        file: PathBuf::from("file.py"),
        line: 10,
        col: 0,
        severity: "HIGH".to_owned(),
    }];
    let result = print_findings(&mut buffer, "Security Issues", &findings);
    assert!(result.is_ok());
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("Security Issues"));
}

#[test]
fn test_print_unused_items_empty() {
    let mut buffer = Vec::new();
    let result = print_unused_items(&mut buffer, "Unused", &[], "Function");
    assert!(result.is_ok());
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.is_empty());
}

#[test]
fn test_print_unused_items_with_items() {
    let mut buffer = Vec::new();
    let items = vec![Definition {
        name: "test_func".to_owned(),
        full_name: "module.test_func".to_owned(),
        simple_name: "test_func".to_owned(),
        def_type: "function".to_owned(),
        file: Arc::new(PathBuf::from("test.py")),
        line: 5,
        end_line: 5,
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
        message: None,
        fix: None,
        is_enum_member: false,
        is_constant: false,
        is_potential_secret: false,
    }];
    let result = print_unused_items(&mut buffer, "Unused Functions", &items, "Function");
    assert!(result.is_ok());
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("Unused Functions"));
}

#[test]
fn test_print_unused_parameters() {
    let mut buffer = Vec::new();
    let items = vec![Definition {
        name: "module.MyClass.method.param".to_owned(),
        full_name: "module.MyClass.method.param".to_owned(),
        simple_name: "param".to_owned(),
        def_type: "parameter".to_owned(),
        file: Arc::new(PathBuf::from("test.py")),
        line: 5,
        end_line: 5,
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
        message: None,
        fix: None,
        is_enum_member: false,
        is_constant: false,
        is_potential_secret: false,
    }];
    let result = print_unused_items(&mut buffer, "Unused Parameters", &items, "Parameter");
    assert!(result.is_ok());
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("param"));
}

#[test]
fn test_print_parse_errors_empty() {
    let mut buffer = Vec::new();
    let result = print_parse_errors(&mut buffer, &[]);
    assert!(result.is_ok());
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.is_empty());
}

#[test]
fn test_print_parse_errors_with_errors() {
    let mut buffer = Vec::new();
    let errors = vec![ParseError {
        file: PathBuf::from("bad.py"),
        error: "Syntax error".to_owned(),
    }];
    let result = print_parse_errors(&mut buffer, &errors);
    assert!(result.is_ok());
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("Parse Errors"));
}

#[test]
fn test_print_report_full() {
    let mut buffer = Vec::new();
    let result = create_mock_result();
    let res = print_report(&mut buffer, &result);
    assert!(res.is_ok());
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("Python Static Analysis"));
    assert!(output.contains("Unreachable"));
}

#[test]
fn test_print_report_no_issues() {
    let mut buffer = Vec::new();
    let result = AnalysisResult {
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
        analysis_summary: AnalysisSummary {
            total_files: 0,
            total_lines_analyzed: 0,
            average_complexity: 0.0,
            average_mi: 0.0,
            secrets_count: 0,
            danger_count: 0,
            quality_count: 0,
            taint_count: 0,
            parse_errors_count: 0,
            total_definitions: 0,
            total_directories: 0,
            total_size: 0.0,
            functions_count: 0,
            classes_count: 0,
            raw_metrics: cytoscnpy::raw_metrics::RawMetrics::default(),
            halstead_metrics: cytoscnpy::halstead::HalsteadMetrics::default(),
        },
    };
    let res = print_report(&mut buffer, &result);
    assert!(res.is_ok());
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("All clean"));
}

#[test]
fn test_print_report_quiet() {
    let mut buffer = Vec::new();
    let result = create_mock_result();
    let res = print_report_quiet(&mut buffer, &result);
    assert!(res.is_ok());
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("SUMMARY"));
    // Should not contain detailed tables
    assert!(!output.contains("Unreachable Functions"));
}
