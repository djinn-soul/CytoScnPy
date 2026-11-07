use super::*;

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
