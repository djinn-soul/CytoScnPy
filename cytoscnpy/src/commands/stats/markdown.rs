use super::model::{FileMetrics, ScanOptions, StatsReport};
use std::path::Path;

pub(super) fn generate_markdown_report_v2(
    report: &StatsReport,
    analysis_result: Option<&crate::analyzer::AnalysisResult>,
    file_metrics: &[FileMetrics],
    options: ScanOptions,
) -> String {
    generate_markdown_report(report, analysis_result, file_metrics, options)
}

fn generate_markdown_report(
    report: &StatsReport,
    analysis_result: Option<&crate::analyzer::AnalysisResult>,
    file_metrics: &[FileMetrics],
    options: ScanOptions,
) -> String {
    let mut md = String::new();
    append_overview_section(&mut md, report);

    if options.all {
        append_per_file_metrics_section(&mut md, file_metrics);
    }

    if options.include_secrets() {
        append_finding_section(
            &mut md,
            "Secrets Scan",
            "No secrets detected.",
            analysis_result.map_or_else(Vec::new, |result| {
                result
                    .secrets
                    .iter()
                    .map(|s| {
                        (
                            short_display_name(&s.file),
                            s.line,
                            s.message.as_str().to_owned(),
                        )
                    })
                    .collect()
            }),
        );
    }

    if options.include_danger() {
        append_finding_section(
            &mut md,
            "Dangerous Code",
            "No dangerous code patterns detected.",
            analysis_result.map_or_else(Vec::new, |result| {
                result
                    .danger
                    .iter()
                    .map(|d| {
                        (
                            short_display_name(&d.file),
                            d.line,
                            d.message.as_str().to_owned(),
                        )
                    })
                    .collect()
            }),
        );
    }

    if options.include_quality() {
        append_finding_section(
            &mut md,
            "Quality Issues",
            "No quality issues detected.",
            analysis_result.map_or_else(Vec::new, |result| {
                result
                    .quality
                    .iter()
                    .map(|q| {
                        (
                            short_display_name(&q.file),
                            q.line,
                            q.message.as_str().to_owned(),
                        )
                    })
                    .collect()
            }),
        );
    }

    md
}

fn append_overview_section(md: &mut String, report: &StatsReport) {
    md.push_str("# CytoScnPy Project Statistics Report\n\n");
    md.push_str("## Overview\n\n");
    md.push_str("| Metric              |        Value |\n");
    md.push_str("|---------------------|-------------:|\n");
    md.push_str(&format!(
        "| Total Files         | {:>12} |\n",
        report.total_files
    ));
    md.push_str(&format!(
        "| Total Directories   | {:>12} |\n",
        report.total_directories
    ));
    md.push_str(&format!(
        "| Total Size          | {:>9.2} KB |\n",
        report.total_size_kb
    ));
    md.push_str(&format!(
        "| Total Lines         | {:>12} |\n",
        report.total_lines
    ));
    md.push_str(&format!(
        "| Code Lines          | {:>12} |\n",
        report.code_lines
    ));
    md.push_str(&format!(
        "| Comment Lines       | {:>12} |\n",
        report.comment_lines
    ));
    md.push_str(&format!(
        "| Empty Lines         | {:>12} |\n",
        report.empty_lines
    ));
    md.push_str(&format!(
        "| Functions           | {:>12} |\n",
        report.total_functions
    ));
    md.push_str(&format!(
        "| Classes             | {:>12} |\n",
        report.total_classes
    ));
}

fn append_per_file_metrics_section(md: &mut String, file_metrics: &[FileMetrics]) {
    md.push_str("\n## Per-File Metrics\n\n");
    md.push_str("| File | Code | Comments | Empty | Total | Size (KB) |\n");
    md.push_str("|------|------|----------|-------|-------|----------|\n");
    for file_metric in file_metrics {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:.2} |\n",
            short_display_name(Path::new(&file_metric.file)),
            file_metric.code_lines,
            file_metric.comment_lines,
            file_metric.empty_lines,
            file_metric.total_lines,
            file_metric.size_kb
        ));
    }
}

fn append_finding_section(
    md: &mut String,
    heading: &str,
    empty_message: &str,
    rows: Vec<(String, usize, String)>,
) {
    md.push_str(&format!("\n## {heading}\n\n"));
    if rows.is_empty() {
        md.push_str(empty_message);
        md.push('\n');
        return;
    }

    md.push_str("| File | Line | Issue |\n");
    md.push_str("|------|------|-------|\n");
    for (file, line, issue) in rows {
        md.push_str(&format!("| {file} | {line} | {issue} |\n"));
    }
}

fn short_display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}
