//! Terminal ASCII tables and JSON reporters for naming style distribution analysis.

use std::io::Write;
use std::path::Path;

use colored::Colorize;
use comfy_table::{Cell, Color, Table};

use super::types::{NamingDistributionResult, NamingStyle};

/// Prints formatted terminal report for naming style distribution and consistency.
pub fn print_terminal_report<W: Write>(
    result: &NamingDistributionResult,
    base: Option<&Path>,
    mut writer: W,
) -> std::io::Result<()> {
    print_summary_table(result, &mut writer)?;
    writeln!(writer)?;
    print_distribution_table(result, &mut writer)?;

    if !result.outliers.is_empty() {
        writeln!(writer)?;
        print_outliers_table(result, base, &mut writer)?;
    }

    Ok(())
}

fn print_summary_table<W: Write>(
    result: &NamingDistributionResult,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "{}",
        "=== Naming Style Distribution & Consistency Analysis ===".bold()
    )?;

    let mut table = Table::new();
    table.set_header(vec!["Metric", "Value"]);

    table.add_row(vec![
        "Total Definitions Analyzed".to_owned(),
        result.stats.total_identifiers.to_string(),
    ]);
    table.add_row(vec![
        "Exempt Structural Dunders".to_owned(),
        result.stats.dunder_count.to_string(),
    ]);

    table.add_row(vec![
        Cell::new("Dominant Style"),
        Cell::new(result.stats.dominant_style.as_str()).fg(Color::Cyan),
    ]);

    let consistency_score = result.stats.consistency_score();
    let score_cell = if consistency_score >= 85.0 {
        Cell::new(format!("{consistency_score:.1}%")).fg(Color::Green)
    } else if consistency_score >= 70.0 {
        Cell::new(format!("{consistency_score:.1}%")).fg(Color::Yellow)
    } else {
        Cell::new(format!("{consistency_score:.1}%")).fg(Color::Red)
    };
    table.add_row(vec![Cell::new("Style Consistency"), score_cell]);

    let outlier_cell = if result.outliers.is_empty() {
        Cell::new("0 (clean)").fg(Color::Green)
    } else if result.outliers.len() <= 5 {
        Cell::new(result.outliers.len().to_string()).fg(Color::Yellow)
    } else {
        Cell::new(result.outliers.len().to_string()).fg(Color::Red)
    };
    table.add_row(vec![Cell::new("Outliers Detected"), outlier_cell]);

    writeln!(writer, "{table}")?;
    Ok(())
}

fn print_distribution_table<W: Write>(
    result: &NamingDistributionResult,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(writer, "{}", "--- Style Distribution ---".bold())?;

    let mut table = Table::new();
    table.set_header(vec!["Style", "Count", "Share", "Status"]);

    let styles = [
        NamingStyle::SnakeCase,
        NamingStyle::CamelCase,
        NamingStyle::PascalCase,
        NamingStyle::ScreamingSnakeCase,
        NamingStyle::Mixed,
    ];

    for style in styles {
        let count = result.stats.count_for_style(style);
        let pct = result.stats.percentage_for_style(style);
        let (role_text, role_color) = if count > 0 && style == result.stats.dominant_style {
            ("Dominant", Color::Green)
        } else if count > 0 {
            ("Non-conforming", Color::Yellow)
        } else {
            ("-", Color::DarkGrey)
        };

        table.add_row(vec![
            Cell::new(style.as_str()),
            Cell::new(count.to_string()),
            Cell::new(format!("{pct:.1}%")),
            Cell::new(role_text).fg(role_color),
        ]);
    }

    writeln!(writer, "{table}")?;
    Ok(())
}

/// Maximum number of non-conforming identifier outliers displayed in the terminal table.
const MAX_DISPLAYED_OUTLIERS: usize = 25;

fn print_outliers_table<W: Write>(
    result: &NamingDistributionResult,
    base: Option<&Path>,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "{}",
        "--- Non-Conforming Identifier Outliers ---".bold()
    )?;

    let mut table = Table::new();
    table.set_header(vec![
        "Identifier",
        "Location",
        "Detected Style",
        "Expected Style",
    ]);

    for outlier in result.outliers.iter().take(MAX_DISPLAYED_OUTLIERS) {
        let loc = format!("{}:{}", display_relative(&outlier.file, base), outlier.line);
        table.add_row(vec![
            Cell::new(&outlier.name).fg(Color::Yellow),
            Cell::new(loc),
            Cell::new(outlier.detected_style.as_str()).fg(Color::Red),
            Cell::new(outlier.expected_style.as_str()).fg(Color::Green),
        ]);
    }

    writeln!(writer, "{table}")?;
    if result.outliers.len() > MAX_DISPLAYED_OUTLIERS {
        writeln!(
            writer,
            "... and {} more outlier identifiers",
            result.outliers.len() - MAX_DISPLAYED_OUTLIERS
        )?;
    }
    Ok(())
}

fn display_relative(path: &Path, base: Option<&Path>) -> String {
    if let Some(base_path) = base {
        if let Ok(rel) = path.strip_prefix(base_path) {
            return rel.display().to_string();
        }
    }
    path.display().to_string()
}

/// Serializes naming distribution results to formatted JSON.
pub fn print_json_report<W: Write>(
    result: &NamingDistributionResult,
    mut writer: W,
) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(result)?;
    writeln!(writer, "{json}")?;
    Ok(())
}
