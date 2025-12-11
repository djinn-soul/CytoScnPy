use crate::analyzer::{AnalysisResult, AnalysisSummary};
use crate::rules::Finding;
use crate::visitor::Definition;
use colored::Colorize;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::time::Duration;

/// Print the exclusion list in styled format
pub fn print_exclusion_list(writer: &mut impl Write, folders: &[String]) -> std::io::Result<()> {
    if folders.is_empty() {
        writeln!(writer, "{}", "[OK] No folders excluded".green())?;
    } else {
        let list = folders
            .iter()
            .map(std::string::String::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(writer, "{} {}", "Excluding:".yellow().bold(), list)?;
    }
    Ok(())
}

/// Create and return a spinner for analysis
pub fn create_spinner() -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan} {msg}")
            .expect("Invalid progress style template"),
    );
    spinner.set_message("CytoScnPy analyzing your code…");
    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner
}

/// Print the main header with box-drawing characters
pub fn print_header(writer: &mut impl Write) -> std::io::Result<()> {
    writeln!(writer)?;
    writeln!(
        writer,
        "{}",
        "╔════════════════════════════════════════╗".cyan()
    )?;
    writeln!(
        writer,
        "{}",
        "║  Python Static Analysis Results       ║".cyan().bold()
    )?;
    writeln!(
        writer,
        "{}",
        "╚════════════════════════════════════════╝".cyan()
    )?;
    writeln!(writer)?;
    Ok(())
}

/// Print summary with colored "pills"
pub fn print_summary_pills(
    writer: &mut impl Write,
    result: &AnalysisResult,
) -> std::io::Result<()> {
    fn pill(label: &str, count: usize) -> String {
        if count == 0 {
            format!("{}: {}", label, count.to_string().green())
        } else {
            format!("{}: {}", label, count.to_string().red().bold())
        }
    }

    // First row: Code issues
    writeln!(
        writer,
        "{}  {}  {}  {}  {}",
        pill("Unreachable", result.unused_functions.len()),
        pill("Imports", result.unused_imports.len()),
        pill("Params", result.unused_parameters.len()),
        pill("Vars", result.unused_variables.len()),
        pill("Classes", result.unused_classes.len()),
    )?;

    // Second row: Security and Quality
    writeln!(
        writer,
        "{}  {}  {}  {}",
        pill("Security", result.danger.len()),
        pill("Secrets", result.secrets.len()),
        pill("Quality", result.quality.len()),
        pill("Taint", result.taint_findings.len()),
    )?;

    writeln!(writer)?;
    Ok(())
}

/// Print analysis statistics (files and lines processed)
pub fn print_analysis_stats(
    writer: &mut impl Write,
    summary: &AnalysisSummary,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "{}",
        format!(
            "Analyzed {} files ({} lines)",
            summary.total_files.to_string().bold(),
            summary.total_lines_analyzed.to_string().bold()
        )
        .dimmed()
    )?;

    if summary.average_complexity > 0.0 || summary.average_mi > 0.0 {
        let complexity_color = if summary.average_complexity > 10.0 {
            colored::Color::Red
        } else {
            colored::Color::Green
        };
        let mi_color = if summary.average_mi < 40.0 {
            colored::Color::Red
        } else {
            colored::Color::Green
        };

        writeln!(
            writer,
            "Average Complexity: {} | Average MI: {}",
            format!("{:.2}", summary.average_complexity)
                .color(complexity_color)
                .bold(),
            format!("{:.2}", summary.average_mi).color(mi_color).bold()
        )?;
    }
    writeln!(writer)?;
    Ok(())
}

/// Helper to create a styled table
fn create_table(headers: Vec<&str>) -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(headers);
    table
}

/// Helper to map severity string to Comfy Table Color
fn get_severity_color(severity: &str) -> Color {
    match severity.to_uppercase().as_str() {
        "CRITICAL" | "HIGH" => Color::Red,
        "MEDIUM" => Color::Yellow,
        "LOW" => Color::Blue,
        _ => Color::White,
    }
}

/// Print a list of findings (Security, Quality, Secrets)
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

    for f in findings {
        let location = format!("{}:{}", f.file.display(), f.line);
        let severity_color = get_severity_color(&f.severity);

        table.add_row(vec![
            Cell::new(&f.rule_id).add_attribute(Attribute::Dim),
            Cell::new(&f.message).add_attribute(Attribute::Bold),
            Cell::new(location),
            Cell::new(&f.severity).fg(severity_color),
        ]);
    }

    writeln!(writer, "{table}")?;
    Ok(())
}

/// Print a list of secrets (special case of findings)
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

    for s in secrets {
        let location = format!("{}:{}", s.file.display(), s.line);
        let severity_color = get_severity_color(&s.severity);

        table.add_row(vec![
            Cell::new(&s.rule_id).add_attribute(Attribute::Dim),
            Cell::new(&s.message).add_attribute(Attribute::Bold),
            Cell::new(location),
            Cell::new(&s.severity).fg(severity_color),
        ]);
    }

    writeln!(writer, "{table}")?;
    Ok(())
}

/// Print a list of unused items (Functions, Imports, etc.)
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
            // For parameters, show "param in function"
            let parts: Vec<&str> = item.name.rsplitn(2, '.').collect();
            let function_name = parts.get(1).unwrap_or(&"unknown");
            format!("{} in {}", item.simple_name, function_name)
        } else {
            item.name.clone()
        };

        let location = format!("{}:{}", item.file.display(), item.line);

        table.add_row(vec![
            Cell::new(item_type_label),
            Cell::new(name_display).add_attribute(Attribute::Bold),
            Cell::new(location),
        ]);
    }

    writeln!(writer, "{table}")?;
    Ok(())
}

/// Print a list of parse errors
pub fn print_parse_errors(
    writer: &mut impl Write,
    errors: &[crate::analyzer::ParseError],
) -> std::io::Result<()> {
    if errors.is_empty() {
        return Ok(());
    }

    writeln!(writer, "\n{}", "Parse Errors".bold().underline().red())?;

    let mut table = create_table(vec!["File", "Error"]);

    for e in errors {
        table.add_row(vec![
            Cell::new(e.file.display()).add_attribute(Attribute::Bold),
            Cell::new(&e.error).fg(Color::Red),
        ]);
    }

    writeln!(writer, "{table}")?;
    Ok(())
}

/// Print the full report
pub fn print_report(writer: &mut impl Write, result: &AnalysisResult) -> std::io::Result<()> {
    print_header(writer)?;
    print_summary_pills(writer, result)?;
    print_analysis_stats(writer, &result.analysis_summary)?;
    writeln!(writer)?;

    // Detailed sections
    print_unused_items(
        writer,
        "Unreachable Functions",
        &result.unused_functions,
        "Function",
    )?;
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
    print_parse_errors(writer, &result.parse_errors)?;

    Ok(())
}
