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
use cytoscnpy::visitor::{Definition, DefinitionType};
use smallvec::smallvec;
use std::path::{Path, PathBuf};
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
        missing_dependencies: vec![],
        missing_dependency_details: vec![],
        unused_dependencies: vec![],
        transitive_dependencies: vec![],
        dev_dependencies_in_production: vec![],
        stdlib_dependencies: vec![],
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
        cell_number: None,
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
        cell_number: None,
    });

    result.quality.push(Finding {
        rule_id: "CSP-Q001".to_owned(),
        category: "Maintainability".to_owned(),
        severity: "MEDIUM".to_owned(),
        message: "High complexity".to_owned(),
        file: file.clone(),
        line: 5,
        col: 0,
        cell_number: None,
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
        exploitability_score: 88,
        cell_number: None,
        source_cell_number: None,
    });

    let def_base = Definition {
        name: "".to_owned(),
        full_name: "".to_owned(),
        simple_name: "".to_owned(),
        def_type: DefinitionType::Function,
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
        def_type: DefinitionType::Function,
        message: Some("unused func".into()),
        ..def_base.clone()
    });
    result.unused_classes.push(Definition {
        name: "C".into(),
        full_name: "C".into(),
        simple_name: "C".into(),
        def_type: DefinitionType::Class,
        message: Some("unused class".into()),
        ..def_base.clone()
    });
    result.unused_methods.push(Definition {
        name: "m".into(),
        full_name: "m".into(),
        simple_name: "m".into(),
        def_type: DefinitionType::Method,
        message: Some("unused method".into()),
        ..def_base.clone()
    });
    result.unused_imports.push(Definition {
        name: "i".into(),
        full_name: "i".into(),
        simple_name: "i".into(),
        def_type: DefinitionType::Import,
        message: Some("unused import".into()),
        ..def_base.clone()
    });
    result.unused_variables.push(Definition {
        name: "v".into(),
        full_name: "v".into(),
        simple_name: "v".into(),
        def_type: DefinitionType::Variable,
        message: Some("unused var".into()),
        ..def_base.clone()
    });
    result.unused_parameters.push(Definition {
        name: "p".into(),
        full_name: "p".into(),
        simple_name: "p".into(),
        def_type: DefinitionType::Parameter,
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

#[path = "report_formats_test/cases_1.rs"]
mod cases_1;
