//! Tests for HTML report generation logic.
#![cfg(feature = "html_report")]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use cytoscnpy::analyzer::types::FileMetrics;
use cytoscnpy::analyzer::{AnalysisResult, AnalysisSummary};
use cytoscnpy::report::generator::{generate_report, generate_report_with_theme, ReportTheme};
use cytoscnpy::rules::Finding;
use cytoscnpy::visitor::{Definition, DefinitionType};
use regex::Regex;
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

fn extract_score_and_grade(html: &str) -> (u8, String) {
    let score_re = Regex::new(r"score-value[^>]*>\s*(\d+)\s*</div>").unwrap();
    let grade_re = Regex::new(r"Grade:\s*([A-F])").unwrap();

    let score = score_re
        .captures(html)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<u8>().ok())
        .expect("Expected quality score in dashboard HTML");

    let grade = grade_re
        .captures(html)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_owned())
        .expect("Expected grade in dashboard HTML");

    (score, grade)
}

fn grade_rank(grade: &str) -> u8 {
    match grade {
        "A" => 5,
        "B" => 4,
        "C" => 3,
        "D" => 2,
        _ => 1,
    }
}

fn empty_result() -> AnalysisResult {
    AnalysisResult {
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
        missing_dependencies: vec![],
        missing_dependency_details: vec![],
        unused_dependencies: vec![],
        transitive_dependencies: vec![],
        dev_dependencies_in_production: vec![],
        stdlib_dependencies: vec![],
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
    }
}

fn unused(name: &str, def_type: DefinitionType, line: usize) -> Definition {
    Definition {
        name: name.to_owned(),
        full_name: name.to_owned(),
        simple_name: name.to_owned(),
        def_type,
        file: Arc::new(PathBuf::from("app.py")),
        line,
        confidence: 80,
        ..Default::default()
    }
}

fn finding(rule_id: &str, category: &str, severity: &str, line: usize) -> Finding {
    Finding {
        rule_id: rule_id.to_owned(),
        category: category.to_owned(),
        severity: severity.to_owned(),
        message: format!("{rule_id} message"),
        file: PathBuf::from("app.py"),
        line,
        col: 0,
        cell_number: None,
    }
}

#[path = "report_generator_test/cases_1.rs"]
mod cases_1;
#[path = "report_generator_test/cases_2.rs"]
mod cases_2;
