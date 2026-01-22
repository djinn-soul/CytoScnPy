use crate::analyzer::AnalysisResult;
use crate::rules::Finding;
use std::io::Write;

/// Generates GitHub Actions workflow commands.
///
/// See: https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions
pub fn print_github(writer: &mut impl Write, result: &AnalysisResult) -> std::io::Result<()> {
    print_github_with_root(writer, result, None)
}

/// Generates GitHub Actions workflow commands with an optional root path.
pub fn print_github_with_root(
    writer: &mut impl Write,
    result: &AnalysisResult,
    _root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    // Security Findings
    for finding in &result.danger {
        write_annotation(writer, "error", finding)?;
    }
    // Secrets
    for secret in &result.secrets {
        // Secrets are always errors
        writeln!(
            writer,
            "::error file={},line={},title={}::{}",
            secret.file.to_string_lossy().replace('\\', "/"),
            secret.line,
            secret.rule_id,
            secret.message
        )?;
    }
    // Quality Findings
    for finding in &result.quality {
        write_annotation(writer, "warning", finding)?;
    }
    // Taint Findings
    for finding in &result.taint_findings {
        // Handle TaintFinding manually since it differs from Finding
        writeln!(
            writer,
            "::warning file={},line={},col={},title={}::{} (Source: {})",
            finding.file.to_string_lossy().replace('\\', "/"),
            finding.sink_line,
            finding.sink_col,
            finding.rule_id,
            finding.vuln_type,
            finding.source
        )?;
    }

    // Unused Code - usually warnings
    for func in &result.unused_functions {
        write_unused(
            writer,
            "warning",
            "UnusedFunction",
            &func.file.to_string_lossy(),
            func.line,
            func.col,
            &func.name,
        )?;
    }
    for cls in &result.unused_classes {
        write_unused(
            writer,
            "warning",
            "UnusedClass",
            &cls.file.to_string_lossy(),
            cls.line,
            cls.col,
            &cls.name,
        )?;
    }
    for imp in &result.unused_imports {
        write_unused(
            writer,
            "warning",
            "UnusedImport",
            &imp.file.to_string_lossy(),
            imp.line,
            imp.col,
            &imp.name,
        )?;
    }
    for var in &result.unused_variables {
        write_unused(
            writer,
            "warning",
            "UnusedVariable",
            &var.file.to_string_lossy(),
            var.line,
            var.col,
            &var.name,
        )?;
    }
    for method in &result.unused_methods {
        write_unused(
            writer,
            "warning",
            "UnusedMethod",
            &method.file.to_string_lossy(),
            method.line,
            method.col,
            &method.name,
        )?;
    }
    for param in &result.unused_parameters {
        write_unused(
            writer,
            "warning",
            "UnusedParameter",
            &param.file.to_string_lossy(),
            param.line,
            param.col,
            &param.name,
        )?;
    }

    // Parse Errors
    for error in &result.parse_errors {
        writeln!(
            writer,
            "::error file={},title=ParseError::{}",
            error.file.to_string_lossy().replace('\\', "/"),
            error.error
        )?;
    }

    Ok(())
}

fn write_annotation(
    writer: &mut impl Write,
    level: &str,
    finding: &Finding,
) -> std::io::Result<()> {
    // Map severity to level if needed, but 'level' arg overrides for now based on category
    // GitHub supports: debug, notice, warning, error
    let gh_level = match finding.severity.to_uppercase().as_str() {
        "CRITICAL" | "HIGH" => "error",
        _ => level,
    };

    writeln!(
        writer,
        "::{} file={},line={},col={},title={}::{}",
        gh_level,
        finding.file.to_string_lossy().replace('\\', "/"),
        finding.line,
        finding.col,
        finding.rule_id,
        finding.message
    )?;
    Ok(())
}

fn write_unused(
    writer: &mut impl Write,
    level: &str,
    title: &str,
    file: &str,
    line: usize,
    col: usize,
    name: &str,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "::{} file={},line={},col={},title={}::Unused identifier '{}'",
        level,
        file.replace('\\', "/"),
        line,
        col,
        title,
        name
    )?;
    Ok(())
}
