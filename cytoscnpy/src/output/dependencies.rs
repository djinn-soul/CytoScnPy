use super::tables::{create_table, get_severity_color};
use crate::analyzer::AnalysisResult;
use crate::report::category_findings::collect_dependency_findings;
use colored::Colorize;
use comfy_table::{Attribute, Cell};
use std::io::Write;

const GROUPS: [(&str, &str); 5] = [
    ("CSP-R001", "Missing Dependencies"),
    ("CSP-R002", "Unused Dependencies"),
    ("CSP-R003", "Transitive Dependencies"),
    ("CSP-R004", "Development Dependencies Used in Production"),
    ("CSP-R005", "Standard Library Dependencies"),
];

/// Print all dependency findings grouped by rule.
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn print_dependency_findings(
    writer: &mut impl Write,
    result: &AnalysisResult,
) -> std::io::Result<()> {
    let findings = collect_dependency_findings(result);
    for (rule_id, title) in GROUPS {
        let group: Vec<_> = findings
            .iter()
            .filter(|finding| finding.rule_id == rule_id)
            .collect();
        if group.is_empty() {
            continue;
        }

        writeln!(
            writer,
            "\n{}",
            format!("{title} ({rule_id})").bold().underline()
        )?;
        let mut table = create_table(vec!["Message", "File", "Line", "Severity"]);
        for finding in group {
            let file = finding
                .file
                .as_deref()
                .map(crate::utils::normalize_display_path)
                .unwrap_or_else(|| "-".to_owned());
            let line = finding
                .line
                .map_or_else(|| "-".to_owned(), |line| line.to_string());
            table.add_row(vec![
                Cell::new(&finding.message).add_attribute(Attribute::Bold),
                Cell::new(file),
                Cell::new(line),
                Cell::new(&finding.severity).fg(get_severity_color(&finding.severity)),
            ]);
        }
        writeln!(writer, "{table}")?;
    }
    Ok(())
}
