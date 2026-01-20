//! Tests for HTML report generation logic.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::types::FileMetrics;
use cytoscnpy::analyzer::{AnalysisResult, AnalysisSummary};
use cytoscnpy::report::generator::generate_report;
use cytoscnpy::rules::Finding;
use cytoscnpy::visitor::Definition;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-report-tmp");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("report_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[test]
fn test_generate_report_full() {
    let dir = project_tempdir();
    let output_dir = dir.path();
    let analysis_root = Path::new(".");

    let result = AnalysisResult {
        unused_functions: vec![Definition {
            name: "unused_func".to_owned(),
            full_name: "test.unused_func".to_owned(),
            simple_name: "unused_func".to_owned(),
            def_type: "function".to_owned(),
            file: Arc::new(PathBuf::from("test.py")),
            line: 10,
            end_line: 10,
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
            message: Some("unused".to_owned()),
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
            message: "Function is too complex (McCabe=15)".to_owned(),
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
        file_metrics: vec![FileMetrics {
            file: PathBuf::from("test.py"),
            loc: 100,
            sloc: 80,
            complexity: 15.0,
            mi: 70.0,
            total_issues: 2,
        }],
        analysis_summary: AnalysisSummary {
            total_files: 1,
            secrets_count: 0,
            danger_count: 0,
            quality_count: 1,
            taint_count: 0,
            parse_errors_count: 0,
            total_lines_analyzed: 100,
            total_definitions: 1,
            average_complexity: 15.0,
            average_mi: 70.0,
            total_directories: 0,
            total_size: 1.0,
            functions_count: 1,
            classes_count: 0,
            raw_metrics: cytoscnpy::raw_metrics::RawMetrics::default(),
            halstead_metrics: cytoscnpy::halstead::HalsteadMetrics::default(),
        },
    };

    // Create the mock file so generate_file_views doesn't skip it
    let test_py = PathBuf::from("test.py");
    std::fs::write(&test_py, "def unused_func():\n    pass\n").unwrap();

    let res = generate_report(&result, analysis_root, output_dir);

    // Cleanup the mock file immediately to avoid polluting the repo
    let _ = std::fs::remove_file(&test_py);

    assert!(res.is_ok(), "Report generation failed: {:?}", res.err());

    // Verify files exist
    assert!(output_dir.join("index.html").exists());
    assert!(output_dir.join("issues.html").exists());
    assert!(output_dir.join("files.html").exists());
    assert!(output_dir.join("css/style.css").exists());
    assert!(output_dir.join("js/charts.js").exists());

    // Verify file view was generated (since we created test.py)
    let safe_name = "test.py.html";
    assert!(output_dir.join("files").join(safe_name).exists());
}

#[test]
fn test_calculate_score_logic() {
    let dir = project_tempdir();
    let output_dir = dir.path();
    let analysis_root = Path::new(".");

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
        analysis_summary: AnalysisSummary {
            total_files: 0,
            secrets_count: 0,
            danger_count: 0,
            quality_count: 0,
            taint_count: 0,
            parse_errors_count: 0,
            total_lines_analyzed: 0,
            total_definitions: 0,
            average_complexity: 0.0,
            average_mi: 100.0,
            total_directories: 0,
            total_size: 0.0,
            functions_count: 0,
            classes_count: 0,
            raw_metrics: cytoscnpy::raw_metrics::RawMetrics::default(),
            halstead_metrics: cytoscnpy::halstead::HalsteadMetrics::default(),
        },
    };

    // 1. Perfect score
    generate_report(&result, analysis_root, output_dir).unwrap();
    let html = std::fs::read_to_string(output_dir.join("index.html")).unwrap();
    assert!(html.contains("Grade: A") || html.contains("Grade: B") || html.contains(">A<"));

    // 2. High penalty (Unused code)
    for i in 0..50 {
        result.unused_functions.push(Definition {
            name: format!("f{i}"),
            full_name: format!("f{i}"),
            simple_name: format!("f{i}"),
            def_type: "function".to_owned(),

            file: Arc::new(PathBuf::from("test.py")),
            line: i,
            end_line: i,
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
        });
    }
    generate_report(&result, analysis_root, output_dir).unwrap();
    let _html = std::fs::read_to_string(output_dir.join("index.html")).unwrap();
}
