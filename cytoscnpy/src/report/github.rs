use crate::analyzer::types::ParseError;
use crate::analyzer::AnalysisResult;
use crate::rules::secrets::SecretFinding;
use crate::rules::Finding;
use crate::taint::types::TaintFinding;
use crate::visitor::Definition;
use std::io::Write;

mod categories;

/// Generates `GitHub Actions` workflow commands.
///
/// See: <https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions>
///
/// # Errors
///
/// Returns an error if writing to the `writer` fails.
pub fn print_github(writer: &mut impl Write, result: &AnalysisResult) -> std::io::Result<()> {
    print_github_with_root(writer, result, None)
}

/// Generates `GitHub Actions` workflow commands with an optional root path.
///
/// # Errors
///
/// Returns an error if writing to the `writer` fails.
pub fn print_github_with_root(
    writer: &mut impl Write,
    result: &AnalysisResult,
    root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    write_findings(writer, "error", &result.danger, root)?;
    write_secrets(writer, &result.secrets, root)?;
    write_findings(writer, "warning", &result.quality, root)?;
    write_taint_findings(writer, &result.taint_findings, root)?;
    write_unused_code(writer, result, root)?;
    write_parse_errors(writer, &result.parse_errors, root)?;
    categories::write_extended_findings(writer, result, root)?;

    Ok(())
}

fn write_findings(
    writer: &mut impl Write,
    level: &str,
    findings: &[Finding],
    root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    for finding in findings {
        write_annotation(writer, level, finding, root)?;
    }
    Ok(())
}

fn write_secrets(
    writer: &mut impl Write,
    secrets: &[SecretFinding],
    root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    for secret in secrets {
        let path = normalize_path(&secret.file, root);
        writeln!(
            writer,
            "::error file={},line={},title={}::{}",
            escape_property(&path),
            secret.line,
            escape_property(&secret.rule_id),
            escape_message(&secret.message)
        )?;
    }
    Ok(())
}

fn write_taint_findings(
    writer: &mut impl Write,
    findings: &[TaintFinding],
    root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    for finding in findings {
        let path = normalize_path(&finding.file, root);
        let level = severity_level(&finding.severity.to_string(), "warning");
        writeln!(
            writer,
            "::{level} file={},line={},col={},title={}::{} (Source: {})",
            escape_property(&path),
            finding.sink_line,
            valid_column(finding.sink_col),
            escape_property(&finding.rule_id),
            escape_message(&finding.vuln_type.to_string()),
            escape_message(&finding.source)
        )?;
    }
    Ok(())
}

fn write_unused_code(
    writer: &mut impl Write,
    result: &AnalysisResult,
    root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    write_unused_definitions(writer, "UnusedFunction", &result.unused_functions, root)?;
    write_unused_definitions(writer, "UnusedClass", &result.unused_classes, root)?;
    write_unused_definitions(writer, "UnusedImport", &result.unused_imports, root)?;
    write_unused_definitions(writer, "UnusedVariable", &result.unused_variables, root)?;
    write_unused_definitions(writer, "UnusedMethod", &result.unused_methods, root)?;
    write_unused_definitions(writer, "UnusedParameter", &result.unused_parameters, root)
}

fn write_unused_definitions(
    writer: &mut impl Write,
    title: &str,
    definitions: &[Definition],
    root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    for definition in definitions {
        write_unused(
            writer,
            "warning",
            title,
            definition.file.as_ref(),
            definition.line,
            definition.col,
            &definition.name,
            root,
        )?;
    }
    Ok(())
}

fn write_parse_errors(
    writer: &mut impl Write,
    errors: &[ParseError],
    root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    for error in errors {
        let path = normalize_path(&error.file, root);
        writeln!(
            writer,
            "::error file={}{},title=ParseError::{}",
            escape_property(&path),
            parse_error_line_meta(&error.error),
            escape_message(&error.error)
        )?;
    }
    Ok(())
}

fn parse_error_line_meta(error: &str) -> String {
    error
        .rfind(" at line ")
        .and_then(|idx| error[idx + 9..].parse::<usize>().ok())
        .map(|line| format!(",line={line}"))
        .unwrap_or_default()
}

fn normalize_path(path: &std::path::Path, root: Option<&std::path::Path>) -> String {
    let normalized = if let Some(r) = root {
        // Handle common root cases robustly
        if r.as_os_str() == "." || r.as_os_str().is_empty() {
            path
        } else {
            path.strip_prefix(r).unwrap_or(path)
        }
    } else {
        path
    };
    let s = normalized.to_string_lossy().replace('\\', "/");
    s.strip_prefix("./").unwrap_or(&s).to_owned()
}

fn write_annotation(
    writer: &mut impl Write,
    level: &str,
    finding: &Finding,
    root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    // Map severity to level if needed, but 'level' arg overrides for now based on category
    // GitHub supports: debug, notice, warning, error
    let gh_level = severity_level(&finding.severity, level);

    let path = normalize_path(&finding.file, root);

    writeln!(
        writer,
        "::{} file={},line={},col={},title={}::{} ({}:{})",
        gh_level,
        escape_property(&path),
        finding.line,
        valid_column(finding.col),
        escape_property(&finding.rule_id),
        escape_message(&finding.message),
        escape_message(&path),
        finding.line
    )?;
    Ok(())
}

fn write_unused(
    writer: &mut impl Write,
    level: &str,
    title: &str,
    file: &std::path::Path,
    line: usize,
    col: usize,
    name: &str,
    root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    let path = normalize_path(file, root);
    let message = format!("Unused identifier '{name}' in {path}:{line}");
    writeln!(
        writer,
        "::{level} file={},line={line},col={},title={title}::{}",
        escape_property(&path),
        valid_column(col),
        escape_message(&message)
    )?;
    Ok(())
}

/// Escapes a value for use in a `GitHub` Actions command property.
///
/// Replaces:
/// - `%` with `%25`
/// - `\r` with `%0D`
/// - `\n` with `%0A`
/// - `:` with `%3A`
/// - `,` with `%2C`
pub(super) fn escape_property(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
        .replace(':', "%3A")
        .replace(',', "%2C")
}

/// Escapes a value for use in a `GitHub` Actions command message (data).
///
/// Replaces:
/// - `%` with `%25`
/// - `\r` with `%0D`
/// - `\n` with `%0A`
pub(super) fn escape_message(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

pub(super) fn severity_level<'a>(severity: &str, default: &'a str) -> &'a str {
    match severity.to_uppercase().as_str() {
        "CRITICAL" | "HIGH" => "error",
        _ => default,
    }
}

pub(super) const fn valid_column(column: usize) -> usize {
    if column == 0 {
        1
    } else {
        column
    }
}
