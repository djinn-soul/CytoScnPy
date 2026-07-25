use crate::analyzer::AnalysisResult;
use crate::report::category_findings::{
    collect_clone_findings, collect_dependency_findings, ExtendedFinding,
};
use std::io::Write;
use std::path::Path;

pub(super) fn write_summary_rows(
    writer: &mut impl Write,
    result: &AnalysisResult,
) -> std::io::Result<()> {
    writeln!(writer, "| Clone Findings | {} |", result.clones.len())?;
    writeln!(
        writer,
        "| Missing Dependencies | {} |",
        result.missing_dependencies.len()
    )?;
    writeln!(
        writer,
        "| Unused Dependencies | {} |",
        result.unused_dependencies.len()
    )?;
    writeln!(
        writer,
        "| Transitive Dependencies | {} |",
        result.transitive_dependencies.len()
    )?;
    writeln!(
        writer,
        "| Dev Dependencies in Production | {} |",
        result.dev_dependencies_in_production.len()
    )?;
    writeln!(
        writer,
        "| Standard Library Dependencies | {} |",
        result.stdlib_dependencies.len()
    )
}

pub(super) fn write_sections(
    writer: &mut impl Write,
    result: &AnalysisResult,
    root: Option<&Path>,
) -> std::io::Result<()> {
    write_table(
        writer,
        "Clone Findings",
        &collect_clone_findings(result),
        root,
    )?;
    write_table(
        writer,
        "Dependency Findings",
        &collect_dependency_findings(result),
        root,
    )
}

fn write_table(
    writer: &mut impl Write,
    title: &str,
    findings: &[ExtendedFinding],
    root: Option<&Path>,
) -> std::io::Result<()> {
    if findings.is_empty() {
        return Ok(());
    }
    writeln!(writer, "## {title}\n")?;
    writeln!(writer, "| Rule | File | Line | Message | Severity |")?;
    writeln!(writer, "| --- | --- | ---: | --- | --- |")?;
    for finding in findings {
        let file = finding
            .normalized_path(root)
            .unwrap_or_else(|| "-".to_owned());
        let line = match (finding.line, finding.end_line) {
            (Some(start), Some(end)) => format!("{start}-{end}"),
            (Some(line), None) => line.to_string(),
            _ => "-".to_owned(),
        };
        writeln!(
            writer,
            "| {} | {} | {} | {} | {} |",
            escape_cell(&finding.rule_id),
            escape_cell(&file),
            line,
            escape_cell(&finding.message),
            escape_cell(&finding.severity)
        )?;
    }
    writeln!(writer)
}

fn escape_cell(value: &str) -> String {
    value.replace('|', "\\|").replace(['\r', '\n'], " ")
}
