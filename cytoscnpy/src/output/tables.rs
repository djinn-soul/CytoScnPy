use crate::rules::Finding;
use crate::utils::normalize_display_path;
use crate::visitor::Definition;
use colored::Colorize;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use std::io::Write;

fn create_table(headers: Vec<&str>) -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(headers);

    if cfg!(test) {
        table.set_width(120);
    }
    table
}

fn get_severity_color(severity: &str) -> Color {
    match severity.to_uppercase().as_str() {
        "CRITICAL" | "HIGH" => Color::Red,
        "MEDIUM" => Color::Yellow,
        "LOW" => Color::Blue,
        _ => Color::White,
    }
}

/// Print a list of findings (Security, Quality, Secrets).
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn print_findings(
    writer: &mut impl Write,
    title: &str,
    findings: &[Finding],
) -> std::io::Result<()> {
    if findings.is_empty() {
        return Ok(());
    }

    writeln!(writer, "\n{}", title.bold().underline())?;
    let mut table = create_table(vec!["Rule ID", "Message", "Location", "Severity"]);

    for finding in findings {
        let location = format!("{}:{}", normalize_display_path(&finding.file), finding.line);
        let severity_color = get_severity_color(&finding.severity);

        table.add_row(vec![
            Cell::new(&finding.rule_id).add_attribute(Attribute::Dim),
            Cell::new(&finding.message).add_attribute(Attribute::Bold),
            Cell::new(location),
            Cell::new(&finding.severity).fg(severity_color),
        ]);
    }

    writeln!(writer, "{table}")?;
    Ok(())
}

/// Print a list of taint analysis findings.
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn print_taint_findings(
    writer: &mut impl Write,
    title: &str,
    findings: &[crate::taint::TaintFinding],
) -> std::io::Result<()> {
    if findings.is_empty() {
        return Ok(());
    }

    writeln!(writer, "\n{}", title.bold().underline())?;
    let mut table = create_table(vec!["Rule ID", "Message", "Location", "Severity"]);

    for finding in findings {
        let location = format!(
            "{}:{}",
            normalize_display_path(&finding.file),
            finding.sink_line
        );
        let severity_str = finding.severity.to_string();
        let severity_color = get_severity_color(&severity_str);

        table.add_row(vec![
            Cell::new(&finding.rule_id).add_attribute(Attribute::Dim),
            Cell::new(format!(
                "{} (Source: {})",
                finding.vuln_type, finding.source
            ))
            .add_attribute(Attribute::Bold),
            Cell::new(location),
            Cell::new(&severity_str).fg(severity_color),
        ]);
    }

    writeln!(writer, "{table}")?;
    Ok(())
}

/// Print a list of secrets (special case of findings).
///
/// # Errors
///
/// Returns an error if writing to the writer fails.
pub fn print_secrets(
    writer: &mut impl Write,
    title: &str,
    secrets: &[crate::rules::secrets::SecretFinding],
) -> std::io::Result<()> {
    if secrets.is_empty() {
        return Ok(());
    }

    writeln!(writer, "\n{}", title.bold().underline())?;
    let mut table = create_table(vec!["Rule ID", "Message", "Location", "Severity"]);

    for secret in secrets {
        let location = format!("{}:{}", normalize_display_path(&secret.file), secret.line);
        let severity_color = get_severity_color(&secret.severity);

        table.add_row(vec![
            Cell::new(&secret.rule_id).add_attribute(Attribute::Dim),
            Cell::new(&secret.message).add_attribute(Attribute::Bold),
            Cell::new(location),
            Cell::new(&secret.severity).fg(severity_color),
        ]);
    }

    writeln!(writer, "{table}")?;
    Ok(())
}

/// Print a list of unused items (Functions, Imports, etc.).
///
/// # Errors
///
/// Returns an error if writing to the writer fails.
pub fn print_unused_items(
    writer: &mut impl Write,
    title: &str,
    items: &[Definition],
    item_type_label: &str,
) -> std::io::Result<()> {
    if items.is_empty() {
        return Ok(());
    }

    writeln!(writer, "\n{}", title.bold().underline())?;
    let mut table = create_table(vec!["Type", "Name", "Location"]);

    for item in items {
        let name_display = if item_type_label == "Parameter" {
            let parts: Vec<&str> = item.name.rsplitn(2, '.').collect();
            let function_part = parts.get(1).unwrap_or(&"unknown");
            let simple_fn: String = function_part
                .rsplit('.')
                .take(2)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join(".");
            format!("{} in {}", item.simple_name, simple_fn)
        } else {
            item.simple_name.clone()
        };

        let location = format!("{}:{}", normalize_display_path(&item.file), item.line);
        table.add_row(vec![
            Cell::new(item_type_label),
            Cell::new(name_display).add_attribute(Attribute::Bold),
            Cell::new(location),
        ]);
    }

    writeln!(writer, "{table}")?;
    Ok(())
}

/// Print a list of parse errors.
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn print_parse_errors(
    writer: &mut impl Write,
    errors: &[crate::analyzer::ParseError],
) -> std::io::Result<()> {
    if errors.is_empty() {
        return Ok(());
    }

    writeln!(writer, "\n{}", "Parse Errors".bold().underline().red())?;
    let mut table = create_table(vec!["File", "Error"]);

    for error in errors {
        table.add_row(vec![
            Cell::new(normalize_display_path(&error.file)).add_attribute(Attribute::Bold),
            Cell::new(&error.error).fg(Color::Red),
        ]);
    }

    writeln!(writer, "{table}")?;
    Ok(())
}
