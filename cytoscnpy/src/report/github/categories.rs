use super::{escape_message, escape_property, valid_column};
use crate::analyzer::AnalysisResult;
use crate::report::category_findings::{collect_extended_findings, ExtendedFinding};
use std::io::Write;
use std::path::Path;

pub(super) fn write_extended_findings(
    writer: &mut impl Write,
    result: &AnalysisResult,
    root: Option<&Path>,
) -> std::io::Result<()> {
    for finding in collect_extended_findings(result) {
        write_finding(writer, &finding, root)?;
    }
    Ok(())
}

fn write_finding(
    writer: &mut impl Write,
    finding: &ExtendedFinding,
    root: Option<&Path>,
) -> std::io::Result<()> {
    let level = annotation_level(&finding.severity);
    let title = escape_property(&finding.rule_id);
    let message = escape_message(&finding.message);
    let Some(path) = finding.normalized_path(root) else {
        return writeln!(writer, "::{level} title={title}::{message}");
    };
    let path = escape_property(&path);

    match (finding.line, finding.column, finding.end_line) {
        (Some(line), Some(column), _) => writeln!(
            writer,
            "::{level} file={path},line={line},col={},title={title}::{message}",
            valid_column(column)
        ),
        (Some(line), None, Some(end_line)) => writeln!(
            writer,
            "::{level} file={path},line={line},endLine={end_line},title={title}::{message}"
        ),
        (Some(line), None, None) => writeln!(
            writer,
            "::{level} file={path},line={line},title={title}::{message}"
        ),
        (None, _, _) => writeln!(writer, "::{level} file={path},title={title}::{message}"),
    }
}

fn annotation_level(severity: &str) -> &str {
    match severity.to_uppercase().as_str() {
        "CRITICAL" | "HIGH" => "error",
        "MEDIUM" | "WARNING" => "warning",
        _ => "notice",
    }
}
