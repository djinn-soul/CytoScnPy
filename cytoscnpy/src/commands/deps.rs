use crate::deps::{analyze_dependencies, DepsOptions};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Cell, Color, Table};
use serde_json::json;

/// Executes the deps subcommand — v3 edition.
/// Reports unused, missing, extra-installed, orphan, and removable-branch findings.
pub fn run_deps<W: std::io::Write>(options: &DepsOptions<'_>, writer: &mut W) -> Result<()> {
    let result = analyze_dependencies(options);

    if options.json {
        let out = json!({
            "unused": result.unused.iter().map(|d| d.package_name.clone()).collect::<Vec<_>>(),
            "missing": result.missing,
            "extra_installed": result.extra_installed.iter().map(|p| json!({
                "name": p.name,
                "version": p.version,
            })).collect::<Vec<_>>(),
            "orphan_installed": result.orphan_installed.iter().map(|p| json!({
                "name": p.name,
                "version": p.version,
            })).collect::<Vec<_>>(),
            "removable_branches": result.removable_branches.iter().map(|b| json!({
                "root": b.root,
                "unique_transitive": b.unique_transitive,
            })).collect::<Vec<_>>(),
        });
        writeln!(writer, "{}", serde_json::to_string_pretty(&out)?)?;
        return Ok(());
    }

    // ── Unused declared ───────────────────────────────────────────────────────
    if !result.unused.is_empty() {
        writeln!(writer, "\n{}", "Unused Dependencies".red().bold())?;
        let mut table = Table::new();
        table.load_preset(UTF8_FULL).set_header(vec![
            "Package Name",
            "Declared In",
            "Type",
            "Confidence",
        ]);

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
                Cell::new("High"),
            ]);
        }
        writeln!(writer, "{table}")?;
    }

    // ── Missing declared ──────────────────────────────────────────────────────
    if !result.missing.is_empty() {
        writeln!(
            writer,
            "\n{}",
            "Missing Dependencies (Imported but not declared)"
                .red()
                .bold()
        )?;
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_header(vec!["Import Name", "Confidence"]);

        for missing in &result.missing {
            table.add_row(vec![
                Cell::new(missing).fg(Color::Yellow),
                Cell::new("High"),
            ]);
        }
        writeln!(writer, "{table}")?;
    }

    // ── Extra installed ───────────────────────────────────────────────────────
    if !result.extra_installed.is_empty() {
        writeln!(
            writer,
            "\n{}",
            "Extra Installed (installed but not declared)"
                .yellow()
                .bold()
        )?;
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_header(vec!["Package", "Version", "Confidence"]);

        for pkg in &result.extra_installed {
            table.add_row(vec![
                Cell::new(&pkg.name).fg(Color::Yellow),
                Cell::new(&pkg.version),
                Cell::new("High"),
            ]);
        }
        writeln!(writer, "{table}")?;
    }

    // ── Orphan installed ──────────────────────────────────────────────────────
    if !result.orphan_installed.is_empty() {
        writeln!(writer, "\n{}", "Orphan Packages (zombie deps)".red().bold())?;
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_header(vec!["Package", "Version", "Confidence"]);

        for pkg in &result.orphan_installed {
            table.add_row(vec![
                Cell::new(&pkg.name).fg(Color::Red),
                Cell::new(&pkg.version),
                Cell::new("High"),
            ]);
        }
        writeln!(writer, "{table}")?;
    }

    // ── Removable branches ────────────────────────────────────────────────────
    if !result.removable_branches.is_empty() {
        writeln!(
            writer,
            "\n{}",
            "Removable Dependency Branches".cyan().bold()
        )?;
        for branch in &result.removable_branches {
            if branch.unique_transitive.is_empty() {
                writeln!(
                    writer,
                    "  {} — safe to remove, no unique transitive deps",
                    branch.root.yellow()
                )?;
            } else {
                writeln!(
                    writer,
                    "  {} — removing this would also allow removing:",
                    branch.root.yellow()
                )?;
                for dep in &branch.unique_transitive {
                    writeln!(writer, "    · {dep}")?;
                }
            }
        }
    }

    // ── Summary ───────────────────────────────────────────────────────────────
    let all_clean = result.unused.is_empty()
        && result.missing.is_empty()
        && result.extra_installed.is_empty()
        && result.orphan_installed.is_empty();

    if all_clean {
        writeln!(
            writer,
            "{}",
            "No unused, missing, extra, or orphan dependencies found!".green()
        )?;
    } else {
        writeln!(
            writer,
            "\nFound: {} unused, {} missing, {} extra installed, {} orphan.",
            result.unused.len(),
            result.missing.len(),
            result.extra_installed.len(),
            result.orphan_installed.len(),
        )?;
    }

    Ok(())
}
