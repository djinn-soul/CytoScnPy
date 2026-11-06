//! Tests for HTML report generation logic.
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
            message: Some("unused".to_owned()),
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
        danger: vec![],
        quality: vec![Finding {
            message: "Function is too complex (McCabe=15)".to_owned(),
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
        file_metrics: vec![FileMetrics {
            file: PathBuf::from("test.py"),
            loc: 100,
            sloc: 80,
            complexity: 15.0,
            mi: 70.0,
            total_issues: 2,
        }],
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
    assert!(output_dir.join("css/prism.css").exists());
    assert!(output_dir.join("js/theme.js").exists());
    assert!(output_dir.join("js/prism.js").exists());

    // Verify file view was generated (since we created test.py)
    let views: Vec<_> = fs::read_dir(output_dir.join("files"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(views.len(), 1);
    assert!(fs::read_to_string(&views[0])
        .unwrap()
        .contains("unused_func"));
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
    };

    // 1. Perfect score
    generate_report(&result, analysis_root, output_dir).unwrap();
    let html = std::fs::read_to_string(output_dir.join("index.html")).unwrap();
    assert!(html.contains("Grade: A") || html.contains("Grade: B") || html.contains(">A<"));
    let (baseline_score, baseline_grade) = extract_score_and_grade(&html);

    // 2. High penalty (Unused code)
    for i in 0..50 {
        result.unused_functions.push(Definition {
            name: format!("f{i}"),
            full_name: format!("f{i}"),
            simple_name: format!("f{i}"),
            def_type: DefinitionType::Function,

            file: Arc::new(PathBuf::from("test.py")),
            line: i,
            end_line: i,
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
            is_unreachable: false,
            category: cytoscnpy::visitor::UnusedCategory::default(),
        });
    }
    generate_report(&result, analysis_root, output_dir).unwrap();
    let html = std::fs::read_to_string(output_dir.join("index.html")).unwrap();
    let (penalty_score, penalty_grade) = extract_score_and_grade(&html);

    assert!(
        penalty_score < baseline_score,
        "Expected degraded score after high-penalty input, baseline={baseline_score}, penalty={penalty_score}"
    );
    assert!(
        grade_rank(&penalty_grade) <= grade_rank(&baseline_grade),
        "Expected degraded or equal grade after penalties, baseline={baseline_grade}, penalty={penalty_grade}"
    );
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

#[test]
fn test_report_pages_use_shared_theme() {
    let dir = project_tempdir();
    let output_dir = dir.path();
    generate_report(&empty_result(), Path::new("."), output_dir).unwrap();

    let hex_color = Regex::new(r"#[0-9a-fA-F]{6}\b").unwrap();
    for page in [
        "index.html",
        "issues.html",
        "files.html",
        "clones.html",
        "deslop.html",
    ] {
        let html = fs::read_to_string(output_dir.join(page)).unwrap();
        assert!(html.contains("css/style.css"), "{page} must link the theme");
        assert!(
            html.contains("js/theme.js"),
            "{page} must load the theme script"
        );
        assert!(
            html.contains("id=\"theme-toggle\""),
            "{page} needs a theme toggle"
        );
        assert!(
            !hex_color.is_match(&html),
            "{page} hardcodes a color instead of using theme variables"
        );
    }

    let css = fs::read_to_string(output_dir.join("css/style.css")).unwrap();
    assert!(css.contains(":root[data-theme=\"light\"]"));
    assert!(css.contains("prefers-color-scheme: light"));
    assert!(css.contains("@media print"));

    // Auto leaves the choice to the viewer's system preference.
    let js = fs::read_to_string(output_dir.join("js/theme.js")).unwrap();
    assert!(js.contains("var DEFAULT_THEME = \"auto\";"));
}

#[test]
fn test_report_theme_default_is_configurable() {
    for (theme, name) in [(ReportTheme::Dark, "dark"), (ReportTheme::Light, "light")] {
        let dir = project_tempdir();
        let output_dir = dir.path();
        generate_report_with_theme(&empty_result(), Path::new("."), output_dir, theme).unwrap();

        let js = fs::read_to_string(output_dir.join("js/theme.js")).unwrap();
        assert!(js.contains(&format!("var DEFAULT_THEME = \"{name}\";")));
        assert!(!js.contains("__CYTOSCNPY_DEFAULT_THEME__"));
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

#[test]
fn test_issues_page_shows_rules_severities_and_all_dead_code() {
    let dir = project_tempdir();
    let output_dir = dir.path();
    let mut result = empty_result();
    result.unused_methods = vec![unused("Svc.run", DefinitionType::Method, 3)];
    result.unused_variables = vec![unused("tmp", DefinitionType::Variable, 4)];
    result.unused_parameters = vec![unused("flag", DefinitionType::Parameter, 5)];
    result.danger = vec![
        finding("CSP-D101", "Injection", "CRITICAL", 10),
        finding("CSP-D999", "Filesystem", "LOW", 11),
    ];
    result.taint_findings = vec![cytoscnpy::taint::TaintFinding {
        source: "request.args".to_owned(),
        source_line: 20,
        category: "Taint".to_owned(),
        sink: "cursor.execute".to_owned(),
        rule_id: "CSP-T001".to_owned(),
        sink_line: 24,
        sink_col: 4,
        flow_path: vec!["query".to_owned(), "sql".to_owned()],
        vuln_type: cytoscnpy::taint::VulnType::SqlInjection,
        severity: cytoscnpy::taint::Severity::High,
        file: PathBuf::from("app.py"),
        remediation: "Use parameterized queries.".to_owned(),
        exploitability_score: 72,
        cell_number: None,
        source_cell_number: None,
    }];

    generate_report(&result, Path::new("."), output_dir).unwrap();
    let html = fs::read_to_string(output_dir.join("issues.html")).unwrap();

    // Danger findings keep their own severity instead of a blanket HIGH.
    assert!(html.contains("badge badge-CRITICAL"));
    assert!(html.contains("badge badge-LOW"));
    assert!(html.contains("<option value=\"CRITICAL\">"));

    // Rule IDs and rule-aware suppression comments are shown.
    assert!(html.contains(">CSP-D101<"));
    assert!(html.contains("data-copy=\"# noqa: CSP-D101\""));
    assert!(html.contains("data-copy=\"# noqa: unused-parameter\""));
    assert!(html.contains("danger:CSP-D101:app.py:10:0"));

    // Every dead-code kind is listed, not only functions, classes and imports.
    assert!(html.contains("Unused method: Svc.run"));
    assert!(html.contains("Unused variable: tmp"));
    assert!(html.contains("Unused parameter: flag"));
    assert!(html.contains("80% confidence"));

    // Taint findings point at the sink and show the flow and remediation.
    assert!(html.contains("app.py:24</a>"));
    assert!(html.contains("<li>query</li>"));
    assert!(html.contains("Use parameterized queries."));
    assert!(html.contains("72/100"));
    assert!(html.contains("Taint: SQL Injection"));
}

#[test]
fn test_deslop_page_placeholder_and_standalone() {
    use cytoscnpy::report::generator::generate_deslop_report;
    use cytoscnpy::report::templates::{DeslopDimensionView, DeslopView};

    let view = || DeslopView {
        slop_index: 37,
        verdict: "ACCEPTABLE".to_owned(),
        raw_score: "52.0".to_owned(),
        size_multiplier: "0.71".to_owned(),
        passed_gate: true,
        dimensions: vec![DeslopDimensionView {
            name: "Architecture clarity".to_owned(),
            weight: 15,
            rating: 2,
            contribution: "6.0".to_owned(),
            evidence: "Two import cycles".to_owned(),
        }],
        recommendations: vec![],
        gate_failures: vec![],
        files_discovered: 4,
        files_checked: 4,
        skipped_files: vec![],
    };

    // Without a main report the page stands alone and links only to itself.
    let dir = project_tempdir();
    let page = generate_deslop_report(view(), Path::new("."), dir.path(), None).unwrap();
    let html = fs::read_to_string(&page).unwrap();
    assert!(html.contains("Architecture clarity"));
    assert!(html.contains("ACCEPTABLE"));
    assert!(!html.contains("issues.html"));
    assert!(dir.path().join("css/style.css").exists());

    // The main report adds a placeholder page, then deslop fills it in.
    let dir = project_tempdir();
    generate_report(&empty_result(), Path::new("."), dir.path()).unwrap();
    let placeholder = fs::read_to_string(dir.path().join("deslop.html")).unwrap();
    assert!(placeholder.contains("cytoscnpy deslop . --html"));
    generate_deslop_report(view(), Path::new("."), dir.path(), None).unwrap();
    let html = fs::read_to_string(dir.path().join("deslop.html")).unwrap();
    assert!(html.contains("Two import cycles"));
    assert!(html.contains("issues.html"));

    // Regenerating the main report keeps the real page.
    generate_report(&empty_result(), Path::new("."), dir.path()).unwrap();
    let html = fs::read_to_string(dir.path().join("deslop.html")).unwrap();
    assert!(html.contains("Two import cycles"));
}

#[test]
fn test_deslop_page_gains_navigation_and_keeps_its_own_theme() {
    use cytoscnpy::report::generator::generate_deslop_report;
    use cytoscnpy::report::templates::DeslopView;

    let view = || DeslopView {
        slop_index: 12,
        verdict: "CLEAN".to_owned(),
        raw_score: "12.0".to_owned(),
        size_multiplier: "1.00".to_owned(),
        passed_gate: true,
        dimensions: vec![],
        recommendations: vec![],
        gate_failures: vec![],
        files_discovered: 1,
        files_checked: 1,
        skipped_files: vec![],
    };

    // A standalone page links to the rest once the main report exists.
    let dir = project_tempdir();
    generate_deslop_report(view(), Path::new("."), dir.path(), None).unwrap();
    generate_report(&empty_result(), Path::new("."), dir.path()).unwrap();
    let html = fs::read_to_string(dir.path().join("deslop.html")).unwrap();
    assert!(html.contains("CLEAN"));
    assert!(html.contains("issues.html"));

    // An explicit theme applies to the page without changing the report's.
    generate_deslop_report(view(), Path::new("."), dir.path(), Some(ReportTheme::Light)).unwrap();
    let html = fs::read_to_string(dir.path().join("deslop.html")).unwrap();
    assert!(html.contains(r#"data-default-theme="light""#));
    let index = fs::read_to_string(dir.path().join("index.html")).unwrap();
    assert!(!index.contains("data-default-theme"));
}

#[test]
fn test_file_view_messages_cannot_break_its_script() {
    let dir = project_tempdir();
    let source = dir.path().join("app.py");
    fs::write(&source, "x = 1\n").unwrap();
    let output_dir = dir.path().join("report");

    let mut result = empty_result();
    result.file_metrics = vec![FileMetrics {
        file: source.clone(),
        loc: 1,
        sloc: 1,
        complexity: 1.0,
        mi: 100.0,
        total_issues: 1,
    }];
    let mut bad = finding("CSP-D001", "Injection", "HIGH", 1);
    bad.file = source;
    bad.message = "Avoid 'eval' \\\" \n </script><script>alert(1)</script>".to_owned();
    result.danger = vec![bad];

    generate_report(&result, Path::new("."), &output_dir).unwrap();
    let view = fs::read_dir(output_dir.join("files"))
        .unwrap()
        .map(|entry| fs::read_to_string(entry.unwrap().path()).unwrap())
        .next()
        .unwrap();
    // The message lives in an escaped attribute, never in script source.
    assert!(view.contains("data-message=\"[CSP-D001] Avoid &#39;eval&#39;"));
    assert_eq!(view.matches("<script>alert(1)").count(), 0);
}
