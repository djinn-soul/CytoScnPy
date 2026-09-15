//! Terminal and JSON reporters for codebase searchability analysis.

use std::io::Write;
use std::path::Path;

use colored::Colorize;
use comfy_table::{Cell, Color, Table};

use super::filenames::display_relative;
use super::types::{GenericCategory, SearchabilityResult};

/// Prints formatted terminal report for searchability results.
pub fn print_terminal_report<W: Write>(
    result: &SearchabilityResult,
    base: Option<&Path>,
    mut writer: W,
) -> std::io::Result<()> {
    print_summary_table(result, &mut writer)?;

    if !result.duplicate_files.is_empty() {
        writeln!(writer)?;
        print_duplicate_files_table(result, base, &mut writer)?;
    }

    if !result.function_collisions.is_empty() {
        writeln!(writer)?;
        print_function_collisions_table(result, base, &mut writer)?;
    }

    if !result.generic_names.is_empty() {
        writeln!(writer)?;
        print_generic_names_table(result, base, &mut writer)?;
    }

    Ok(())
}

fn print_summary_table<W: Write>(
    result: &SearchabilityResult,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "{}",
        "=== Codebase Searchability & Name Collision Analysis ===".bold()
    )?;

    let mut table = Table::new();
    table.set_header(vec!["Metric", "Value"]);

    table.add_row(vec![
        "Total Files Scanned".to_owned(),
        result.stats.total_files.to_string(),
    ]);
    table.add_row(vec![
        "Total Functions Analyzed".to_owned(),
        result.stats.total_functions.to_string(),
    ]);

    let dup_badge = if result.stats.duplicate_filenames == 0 {
        Cell::new("0 (Clean)").fg(Color::Green)
    } else {
        Cell::new(result.stats.duplicate_filenames.to_string()).fg(Color::Yellow)
    };
    table.add_row(vec![Cell::new("Duplicate Filenames"), dup_badge]);

    if let Some((name, count)) = &result.stats.worst_duplicate_filename {
        table.add_row(vec![
            "Worst Duplicate Filename".to_owned(),
            format!("{name} ({count} locations)"),
        ]);
    }

    let col_badge = if result.stats.function_name_collisions == 0 {
        Cell::new("0 (Clean)").fg(Color::Green)
    } else {
        Cell::new(result.stats.function_name_collisions.to_string()).fg(Color::Yellow)
    };
    table.add_row(vec![
        Cell::new("Function Name Collisions (>= 3 files)"),
        col_badge,
    ]);

    if let Some((name, count)) = &result.stats.worst_function_collision {
        table.add_row(vec![
            "Worst Function Collision".to_owned(),
            format!("{name} ({count} files)"),
        ]);
    }

    let gen_badge = if result.stats.generic_name_count == 0 {
        Cell::new("0").fg(Color::Green)
    } else {
        Cell::new(result.stats.generic_name_count.to_string()).fg(Color::Cyan)
    };
    table.add_row(vec![Cell::new("Generic Name Occurrences"), gen_badge]);

    writeln!(writer, "{table}")
}

fn print_duplicate_files_table<W: Write>(
    result: &SearchabilityResult,
    base: Option<&Path>,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "{}",
        "--- Duplicate Filenames Across Directories ---".bold()
    )?;

    let mut table = Table::new();
    table.set_header(vec!["Filename", "Count", "Locations"]);

    for item in result.duplicate_files.iter().take(15) {
        let locations = item
            .paths
            .iter()
            .take(3)
            .map(|p| display_relative(p, base))
            .collect::<Vec<_>>()
            .join("\n");
        let extra = if item.paths.len() > 3 {
            format!("\n... (+{} more)", item.paths.len() - 3)
        } else {
            String::new()
        };

        table.add_row(vec![
            Cell::new(&item.filename).fg(Color::Yellow),
            Cell::new(item.count.to_string()),
            Cell::new(format!("{locations}{extra}")),
        ]);
    }

    writeln!(writer, "{table}")?;
    if result.duplicate_files.len() > 15 {
        writeln!(
            writer,
            "... and {} more duplicate filenames",
            result.duplicate_files.len() - 15
        )?;
    }
    Ok(())
}

fn print_function_collisions_table<W: Write>(
    result: &SearchabilityResult,
    base: Option<&Path>,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "{}",
        "--- Function Name Collisions (Defined in >= 3 files) ---".bold()
    )?;

    let mut table = Table::new();
    table.set_header(vec![
        "Function Name",
        "Distinct Files",
        "Sample Definitions",
    ]);

    for item in result.function_collisions.iter().take(15) {
        let samples = item
            .locations
            .iter()
            .take(3)
            .map(|loc| format!("{}:{}", display_relative(&loc.file, base), loc.line))
            .collect::<Vec<_>>()
            .join("\n");
        let extra = if item.locations.len() > 3 {
            format!("\n... (+{} more)", item.locations.len() - 3)
        } else {
            String::new()
        };

        table.add_row(vec![
            Cell::new(&item.function_name).fg(Color::Red),
            Cell::new(item.file_count.to_string()),
            Cell::new(format!("{samples}{extra}")),
        ]);
    }

    writeln!(writer, "{table}")?;
    if result.function_collisions.len() > 15 {
        writeln!(
            writer,
            "... and {} more colliding functions",
            result.function_collisions.len() - 15
        )?;
    }
    Ok(())
}

fn print_generic_names_table<W: Write>(
    result: &SearchabilityResult,
    base: Option<&Path>,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "{}",
        "--- Generic Identifier Occurrences ---".bold()
    )?;

    let mut table = Table::new();
    table.set_header(vec!["Identifier", "Kind", "Location"]);

    for item in result.generic_names.iter().take(15) {
        let kind = match item.category {
            GenericCategory::Filename => "Filename",
            GenericCategory::Function => "Function",
        };
        let loc = match item.line {
            Some(line) => format!("{}:{}", display_relative(&item.file, base), line),
            None => display_relative(&item.file, base),
        };

        table.add_row(vec![
            Cell::new(&item.identifier).fg(Color::Cyan),
            Cell::new(kind),
            Cell::new(loc),
        ]);
    }

    writeln!(writer, "{table}")?;
    if result.generic_names.len() > 15 {
        writeln!(
            writer,
            "... and {} more generic identifiers",
            result.generic_names.len() - 15
        )?;
    }
    Ok(())
}

/// Serializes searchability results to formatted JSON.
pub fn print_json_report<W: Write>(
    result: &SearchabilityResult,
    mut writer: W,
) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(result)?;
    writeln!(writer, "{json}")?;
    Ok(())
}
