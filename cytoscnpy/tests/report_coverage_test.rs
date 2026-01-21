//! Report generator coverage tests.
use cytoscnpy::analyzer::types::FileMetrics;
use cytoscnpy::analyzer::{AnalysisResult, AnalysisSummary};
use cytoscnpy::report::generator::generate_report;
use cytoscnpy::rules::secrets::SecretFinding;
use cytoscnpy::rules::Finding;
use cytoscnpy::visitor::Definition;
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn test_generate_report_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = tempdir()?;
    let root = output_dir.path().to_path_buf();

    // Create a dummy source file so file view generation works
    let dummy_file_path = root.join("test.py");
    std::fs::write(&dummy_file_path, "def main(): pass")?;

    // Construct a complex AnalysisResult that hits all branches
    let mut result = AnalysisResult::default();

    // 1. File Metrics (Complexity penalties)
    result.file_metrics.push(FileMetrics {
        file: dummy_file_path.clone(),
        loc: 600,
        sloc: 501, // > 500 triggers mismatch penalty
        complexity: 15.0,
        mi: 40.0, // Low MI triggers color change
        total_issues: 5,
    });

    // 2. Unused Code (Maintainability penalty)
    result.unused_functions.push(Definition {
        file: Arc::new(dummy_file_path.clone()),
        line: 1,
        name: "unused_func".to_owned(),
        full_name: "test.unused_func".to_owned(),
        def_type: "function".to_owned(),
        confidence: 100,
        col: 0,
        ..Definition::default()
    });

    // 3. Quality Issues (Reliability/Style penalties)
    result.quality.push(Finding {
        file: dummy_file_path.clone(),
        line: 1,
        message: "Function too complex (McCabe=15)".to_owned(),
        category: "Maintainability".to_owned(),
        severity: "HIGH".to_owned(),
        rule_id: "CSP-Q001".to_owned(),
        col: 0,
    });
    result.quality.push(Finding {
        file: dummy_file_path.clone(),
        line: 1,
        message: "Function too long".to_owned(),
        category: "Maintainability".to_owned(),
        severity: "MEDIUM".to_owned(),
        rule_id: "CSP-Q002".to_owned(),
        col: 0,
    });
    result.quality.push(Finding {
        file: dummy_file_path.clone(),
        line: 1,
        message: "Possible panic detection".to_owned(), // Triggers reliability
        category: "Reliability".to_owned(),
        severity: "HIGH".to_owned(),
        rule_id: "CSP-Q003".to_owned(),
        col: 0,
    });
    result.quality.push(Finding {
        file: dummy_file_path.clone(),
        line: 1,
        message: "Bad style".to_owned(), // Triggers style
        category: "Style".to_owned(),
        severity: "LOW".to_owned(),
        rule_id: "CSP-Q004".to_owned(),
        col: 0,
    });

    // 4. Security Issues
    result.secrets.push(SecretFinding {
        file: dummy_file_path,
        line: 1,
        message: "Hardcoded password".to_owned(),
        category: "Secrets".to_owned(),
        severity: "CRITICAL".to_owned(),
        rule_id: "CSP-S001".to_owned(),
        confidence: 100,
        matched_value: None,
        entropy: None,
    });

    // Set some summary values to trigger branches
    result.analysis_summary = AnalysisSummary {
        total_files: 1,
        average_mi: 40.0,
        ..AnalysisSummary::default()
    };

    // Generate Report
    let report_out = output_dir.path().join("report");
    generate_report(&result, &root, &report_out)?;

    // Assert existence of files
    assert!(report_out.join("index.html").exists());
    assert!(report_out.join("issues.html").exists());
    assert!(report_out.join("files.html").exists());
    assert!(report_out.join("clones.html").exists());
    assert!(report_out.join("css/style.css").exists());
    assert!(report_out.join("files").exists());

    // Optional: read issues.html and check for findings
    let issues_html = std::fs::read_to_string(report_out.join("issues.html"))?;
    assert!(issues_html.contains("Security"));
    assert!(issues_html.contains("Hardcoded password"));
    Ok(())
}
