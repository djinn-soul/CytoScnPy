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
use cytoscnpy::visitor::{Definition, DefinitionType};
use std::path::PathBuf;
use std::sync::Arc;

fn create_mock_result() -> AnalysisResult {
    AnalysisResult {
        unused_functions: vec![Definition {
            name: "unused_func".to_owned(),
            full_name: "test.unused_func".to_owned(),
            simple_name: "unused_func".to_owned(),
            def_type: DefinitionType::Function,
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
            is_unreachable: true,
            category: cytoscnpy::visitor::UnusedCategory::default(),
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
            cell_number: None,
        }],
        taint_findings: vec![],
        parse_errors: vec![],
        clones: vec![],
        file_metrics: vec![],
        missing_dependencies: vec![],
        missing_dependency_details: vec![],
        unused_dependencies: vec![],
        transitive_dependencies: vec![],
        dev_dependencies_in_production: vec![],
        stdlib_dependencies: vec![],
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

#[path = "output_formatting_test/cases_1.rs"]
mod cases_1;
#[path = "output_formatting_test/cases_2.rs"]
mod cases_2;
