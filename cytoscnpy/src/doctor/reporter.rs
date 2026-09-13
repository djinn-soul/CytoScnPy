use colored::Colorize;
use comfy_table::{Cell, Color, Table};
use std::io::Write;

use super::types::{DoctorResult, SetupVerdict};

/// Formats and prints the terminal report for doctor results.
pub fn print_terminal_report<W: Write>(
    result: &DoctorResult,
    mut writer: W,
) -> std::io::Result<()> {
    print_reliability_table(result, &mut writer)?;
    writeln!(writer)?;
    print_tooling_table(result, &mut writer)?;
    writeln!(writer)?;
    print_structure_table(result, &mut writer)?;

    if !result.reliability.recommendations.is_empty() {
        writeln!(writer)?;
        print_recommendations(result, &mut writer)?;
    }

    Ok(())
}

fn bool_badge(val: bool) -> Cell {
    if val {
        Cell::new("YES").fg(Color::Green)
    } else {
        Cell::new("NO").fg(Color::Red)
    }
}

fn print_reliability_table<W: Write>(result: &DoctorResult, writer: &mut W) -> std::io::Result<()> {
    writeln!(
        writer,
        "{}",
        "=== Setup Reliability & Repository Health ===".bold()
    )?;

    let mut table = Table::new();
    table.set_header(vec!["Signal", "Status"]);

    let rel = &result.reliability;
    let (score_str, verdict_cell) = match rel.verdict {
        SetupVerdict::Ready => (
            format!("{}/100", rel.score).green().to_string(),
            Cell::new("READY").fg(Color::Green),
        ),
        SetupVerdict::Solid => (
            format!("{}/100", rel.score).cyan().to_string(),
            Cell::new("SOLID").fg(Color::Cyan),
        ),
        SetupVerdict::Incomplete => (
            format!("{}/100", rel.score).yellow().to_string(),
            Cell::new("INCOMPLETE").fg(Color::Yellow),
        ),
        SetupVerdict::AtRisk => (
            format!("{}/100", rel.score).red().to_string(),
            Cell::new("AT RISK").fg(Color::Red),
        ),
    };

    table.add_row(vec![Cell::new("Setup Health Score"), Cell::new(score_str)]);
    table.add_row(vec![Cell::new("Setup Readiness Verdict"), verdict_cell]);
    table.add_row(vec![
        Cell::new("Deterministic Lockfile"),
        bool_badge(rel.has_lockfile),
    ]);
    table.add_row(vec![Cell::new("CI Automation"), bool_badge(rel.has_ci)]);
    table.add_row(vec![Cell::new("Test Framework"), bool_badge(rel.has_tests)]);
    table.add_row(vec![Cell::new("Code Linter"), bool_badge(rel.has_linter)]);
    table.add_row(vec![
        Cell::new("Code Formatter"),
        bool_badge(rel.has_formatter),
    ]);
    table.add_row(vec![
        Cell::new("Static Type Checker"),
        bool_badge(rel.has_type_checker),
    ]);
    table.add_row(vec![
        Cell::new("Container / Docker"),
        bool_badge(rel.has_docker),
    ]);
    table.add_row(vec![
        Cell::new("README Setup Instructions"),
        bool_badge(rel.has_readme_setup),
    ]);
    table.add_row(vec![
        Cell::new("Git Ignore File"),
        bool_badge(rel.has_gitignore),
    ]);

    writeln!(writer, "{table}")
}

fn print_tooling_table<W: Write>(result: &DoctorResult, writer: &mut W) -> std::io::Result<()> {
    writeln!(
        writer,
        "{}",
        "=== Detected Tooling & Configuration Inventory ===".bold()
    )?;

    if result.configs.is_empty() {
        writeln!(
            writer,
            "{}",
            "  No standard tooling configuration files detected.".yellow()
        )?;
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec!["Category", "Tool / Name", "Location / Details"]);

    for config in &result.configs {
        let details = config.details.as_deref().unwrap_or_else(|| {
            config
                .path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("")
        });

        table.add_row(vec![
            Cell::new(config.category.to_string()),
            Cell::new(&config.name),
            Cell::new(details),
        ]);
    }

    writeln!(writer, "{table}")
}

fn print_structure_table<W: Write>(result: &DoctorResult, writer: &mut W) -> std::io::Result<()> {
    writeln!(
        writer,
        "{}",
        "=== Repository Structure & Language Volume ===".bold()
    )?;

    let s = &result.structure;
    let mut table = Table::new();
    table.set_header(vec!["Metric", "Value"]);

    table.add_row(vec![
        "Total Scanned Files".to_owned(),
        format!("{} files ({} lines)", s.total_files, s.total_lines),
    ]);

    let test_pct = s.test_to_source_ratio * 100.0;
    table.add_row(vec![
        "Source vs Test Ratio".to_owned(),
        format!(
            "Source: {} lines | Tests: {} lines ({:.1}%)",
            s.source_lines, s.test_lines, test_pct
        ),
    ]);

    table.add_row(vec![
        "Average File Lines".to_owned(),
        format!("{} lines/file", s.avg_file_lines),
    ]);

    table.add_row(vec![
        "Max Directory Depth".to_owned(),
        format!("{} levels", s.max_directory_depth),
    ]);

    if !s.largest_file_path.is_empty() {
        table.add_row(vec![
            "Largest File".to_owned(),
            format!("{} ({} lines)", s.largest_file_path, s.largest_file_lines),
        ]);
    }

    let lang_summary = s
        .languages
        .iter()
        .map(|l| format!("{}: {} lines", l.language, l.line_count))
        .collect::<Vec<_>>()
        .join(", ");

    if !lang_summary.is_empty() {
        table.add_row(vec!["Languages".to_owned(), lang_summary]);
    }

    writeln!(writer, "{table}")
}

fn print_recommendations<W: Write>(result: &DoctorResult, writer: &mut W) -> std::io::Result<()> {
    writeln!(
        writer,
        "{}",
        "=== Actionable Setup Recommendations ===".bold()
    )?;

    for (idx, rec) in result.reliability.recommendations.iter().enumerate() {
        writeln!(writer, "  {}. {}", idx + 1, rec.cyan())?;
    }

    Ok(())
}

/// Formats and writes the JSON report.
pub fn print_json_report<W: Write>(result: &DoctorResult, mut writer: W) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(result)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    writeln!(writer, "{json}")
}
