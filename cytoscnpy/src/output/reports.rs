use crate::analyzer::AnalysisResult;
use crate::utils::normalize_display_path;
use colored::Colorize;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use super::summary::print_header;
use super::tables::{
    print_findings, print_parse_errors, print_secrets, print_taint_findings, print_unused_items,
};

/// Print the full report.
///
/// # Errors
///
/// Returns an error if writing to the writer fails.
pub fn print_report(writer: &mut impl Write, result: &AnalysisResult) -> std::io::Result<()> {
    print_header(writer)?;

    let total_issues = result.unused_functions.len()
        + result.unused_methods.len()
        + result.unused_imports.len()
        + result.unused_parameters.len()
        + result.unused_classes.len()
        + result.unused_variables.len()
        + result.danger.len()
        + result.secrets.len()
        + result.quality.len()
        + result.taint_findings.len()
        + result.parse_errors.len();

    if total_issues == 0 {
        writeln!(writer, "{}", "✓ All clean! No issues found.".green())?;
        return Ok(());
    }

    print_unused_items(
        writer,
        "Unreachable Functions",
        &result.unused_functions,
        "Function",
    )?;
    print_unused_items(writer, "Unused Methods", &result.unused_methods, "Method")?;
    print_unused_items(writer, "Unused Imports", &result.unused_imports, "Import")?;
    print_unused_items(
        writer,
        "Unused Parameters",
        &result.unused_parameters,
        "Parameter",
    )?;
    print_unused_items(writer, "Unused Classes", &result.unused_classes, "Class")?;
    print_unused_items(
        writer,
        "Unused Variables",
        &result.unused_variables,
        "Variable",
    )?;

    print_findings(writer, "Security Issues", &result.danger)?;
    print_secrets(writer, "Secrets", &result.secrets)?;
    print_findings(writer, "Quality Issues", &result.quality)?;
    print_taint_findings(writer, "Taint Analysis Findings", &result.taint_findings)?;
    print_parse_errors(writer, &result.parse_errors)?;
    Ok(())
}

/// Print a list of findings grouped by file.
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn print_report_grouped(
    writer: &mut impl Write,
    result: &AnalysisResult,
) -> std::io::Result<()> {
    print_header(writer)?;
    let mut grouped: BTreeMap<String, Vec<(usize, String, String)>> = BTreeMap::new();

    let mut add = |file: &str, line: usize, msg: String, severity: &str| {
        grouped
            .entry(file.to_owned())
            .or_default()
            .push((line, msg, severity.to_owned()));
    };

    for finding in &result.danger {
        add(
            &finding.file.to_string_lossy(),
            finding.line,
            format!("[SECURITY] {}", finding.message),
            &finding.severity,
        );
    }
    for secret in &result.secrets {
        add(
            &secret.file.to_string_lossy(),
            secret.line,
            format!("[SECRET] {}", secret.message),
            &secret.severity,
        );
    }
    for finding in &result.quality {
        add(
            &finding.file.to_string_lossy(),
            finding.line,
            format!("[QUALITY] {}", finding.message),
            &finding.severity,
        );
    }
    for finding in &result.taint_findings {
        add(
            &finding.file.to_string_lossy(),
            finding.sink_line,
            format!("[TAINT] {} (Source: {})", finding.vuln_type, finding.source),
            &finding.severity.to_string(),
        );
    }
    for item in &result.unused_functions {
        add(
            &item.file.to_string_lossy(),
            item.line,
            format!("[UNUSED] Function '{}'", item.name),
            "LOW",
        );
    }
    for item in &result.unused_methods {
        add(
            &item.file.to_string_lossy(),
            item.line,
            format!("[UNUSED] Method '{}'", item.name),
            "LOW",
        );
    }
    for item in &result.unused_classes {
        add(
            &item.file.to_string_lossy(),
            item.line,
            format!("[UNUSED] Class '{}'", item.name),
            "LOW",
        );
    }
    for item in &result.unused_imports {
        add(
            &item.file.to_string_lossy(),
            item.line,
            format!("[UNUSED] Import '{}'", item.name),
            "LOW",
        );
    }
    for item in &result.unused_variables {
        add(
            &item.file.to_string_lossy(),
            item.line,
            format!("[UNUSED] Variable '{}'", item.simple_name),
            "LOW",
        );
    }
    for item in &result.unused_parameters {
        add(
            &item.file.to_string_lossy(),
            item.line,
            format!("[UNUSED] Parameter '{}'", item.simple_name),
            "LOW",
        );
    }
    for error in &result.parse_errors {
        add(
            &error.file.to_string_lossy(),
            0,
            format!("[ERROR] Parse Error: {}", error.error),
            "HIGH",
        );
    }

    for (file, issues) in grouped {
        writeln!(
            writer,
            "\nFile: {}",
            normalize_display_path(Path::new(&file)).bold().underline()
        )?;
        for (line, msg, severity) in issues {
            let color = match severity.to_uppercase().as_str() {
                "CRITICAL" | "HIGH" => colored::Color::Red,
                "MEDIUM" => colored::Color::Yellow,
                "LOW" => colored::Color::Blue,
                _ => colored::Color::White,
            };
            writeln!(
                writer,
                "  Line {}: {}",
                line.to_string().cyan(),
                msg.color(color)
            )?;
        }
    }

    Ok(())
}

/// Print a quiet report (no detailed tables) for CI/CD mode.
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn print_report_quiet(writer: &mut impl Write, result: &AnalysisResult) -> std::io::Result<()> {
    writeln!(writer)?;

    let total = result.unused_functions.len()
        + result.unused_methods.len()
        + result.unused_imports.len()
        + result.unused_parameters.len()
        + result.unused_classes.len()
        + result.unused_variables.len();
    let security = result.danger.len()
        + result.secrets.len()
        + result.quality.len()
        + result.taint_findings.len();
    writeln!(
        writer,
        "\n[SUMMARY] {total} unused code issues, {security} security/quality issues"
    )?;

    Ok(())
}
