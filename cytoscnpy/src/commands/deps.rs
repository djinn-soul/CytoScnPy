use crate::deps::{analyze_dependencies, DepsOptions};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Cell, Color, Table};
use serde_json::json;

/// Executes the deps subcommand, identifying unused and missing dependencies.
pub fn run_deps<W: std::io::Write>(options: &DepsOptions<'_>, writer: &mut W) -> Result<()> {
    let result = analyze_dependencies(options);

    if options.json {
        let out = json!({
            "unused": result.unused.iter().map(|d| d.package_name.clone()).collect::<Vec<_>>(),
            "missing": result.missing,
        });
        writeln!(writer, "{}", serde_json::to_string_pretty(&out)?)?;
        return Ok(());
    }

    if !result.unused.is_empty() {
        writeln!(writer, "\n{}", "Unused Dependencies".red().bold())?;
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_header(vec!["Package Name", "Declared In", "Type"]);

        for dep in &result.unused {
            let source_str = match &dep.source {
                crate::deps::DependencySource::Pyproject => "pyproject.toml".to_owned(),
                crate::deps::DependencySource::Requirements(f) => f.clone(),
            };
            let dev_str = if dep.is_dev { "dev" } else { "prod" };
            table.add_row(vec![
                Cell::new(&dep.package_name).fg(Color::Yellow),
                Cell::new(source_str),
                Cell::new(dev_str),
            ]);
        }
        writeln!(writer, "{table}")?;
    }

    if !result.missing.is_empty() {
        writeln!(
            writer,
            "\n{}",
            "Missing Dependencies (Imported but not declared)"
                .red()
                .bold()
        )?;
        let mut table = Table::new();
        table.load_preset(UTF8_FULL).set_header(vec!["Import Name"]);

        for missing in &result.missing {
            table.add_row(vec![Cell::new(missing).fg(Color::Yellow)]);
        }
        writeln!(writer, "{table}")?;
    }

    if result.unused.is_empty() && result.missing.is_empty() {
        writeln!(
            writer,
            "{}",
            "No unused or missing dependencies found!".green()
        )?;
    } else {
        writeln!(
            writer,
            "\nFound {} unused and {} missing dependencies.",
            result.unused.len(),
            result.missing.len()
        )?;
    }

    Ok(())
}
