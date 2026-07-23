use super::escape_xml;
use crate::analyzer::AnalysisResult;
use crate::report::category_findings::collect_extended_findings;
use std::io::Write;
use std::path::Path;

pub(super) fn count_findings(result: &AnalysisResult) -> usize {
    collect_extended_findings(result).len()
}

pub(super) fn write_findings(
    writer: &mut impl Write,
    result: &AnalysisResult,
    root: Option<&Path>,
) -> std::io::Result<()> {
    for finding in collect_extended_findings(result) {
        let file = finding
            .normalized_path(root)
            .unwrap_or_else(|| "project".to_owned());
        let line = finding.line.unwrap_or(1);
        writeln!(
            writer,
            "    <testcase name=\"{}\" classname=\"{}\">",
            escape_xml(&format!("{}:{}", finding.category, finding.rule_id)),
            escape_xml(&file)
        )?;
        writeln!(
            writer,
            "      <failure message=\"{}\">Line {}: {} ({}:{})</failure>",
            escape_xml(&finding.message),
            line,
            escape_xml(&finding.message),
            escape_xml(&file),
            line
        )?;
        writeln!(writer, "    </testcase>")?;
    }
    Ok(())
}
