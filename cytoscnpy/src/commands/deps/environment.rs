use crate::deps::DepsResult;
use anyhow::Result;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Cell, Color, Table};
use std::io::Write;

pub(super) fn write_extra_installed<W: Write>(result: &DepsResult, writer: &mut W) -> Result<()> {
    if result.extra_installed.is_empty() {
        return Ok(());
    }

    writeln!(
        writer,
        "\n{}",
        "Extra Installed (installed but not declared)"
            .yellow()
            .bold()
    )?;
    write_package_table(&result.extra_installed, Color::Yellow, writer)
}

pub(super) fn write_orphan_installed<W: Write>(result: &DepsResult, writer: &mut W) -> Result<()> {
    if result.orphan_installed.is_empty() {
        return Ok(());
    }

    writeln!(writer, "\n{}", "Orphan Packages (zombie deps)".red().bold())?;
    write_package_table(&result.orphan_installed, Color::Red, writer)
}

fn write_package_table<W: Write>(
    packages: &[crate::deps::InstalledPackage],
    name_color: Color,
    writer: &mut W,
) -> Result<()> {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec!["Package", "Version"]);

    for pkg in packages {
        table.add_row(vec![
            Cell::new(&pkg.name).fg(name_color),
            Cell::new(&pkg.version),
        ]);
    }
    writeln!(writer, "{table}")?;
    Ok(())
}

pub(super) fn write_removable_branches<W: Write>(
    result: &DepsResult,
    writer: &mut W,
) -> Result<()> {
    if result.removable_branches.is_empty() {
        return Ok(());
    }

    writeln!(
        writer,
        "\n{}",
        "Removable Dependency Branches".cyan().bold()
    )?;
    for branch in &result.removable_branches {
        write_removable_branch(branch, writer)?;
    }
    Ok(())
}

fn write_removable_branch<W: Write>(
    branch: &crate::deps::RemovableBranch,
    writer: &mut W,
) -> Result<()> {
    if branch.unique_transitive.is_empty() {
        write_leaf_removable_branch(&branch.root, writer)?;
    } else {
        write_transitive_removable_branch(branch, writer)?;
    }
    Ok(())
}

fn write_leaf_removable_branch<W: Write>(root: &str, writer: &mut W) -> Result<()> {
    writeln!(
        writer,
        "  {} — safe to remove, no unique transitive deps",
        root.yellow()
    )?;
    Ok(())
}

fn write_transitive_removable_branch<W: Write>(
    branch: &crate::deps::RemovableBranch,
    writer: &mut W,
) -> Result<()> {
    writeln!(
        writer,
        "  {} — removing this would also allow removing:",
        branch.root.yellow()
    )?;
    for dep in &branch.unique_transitive {
        writeln!(writer, "    · {dep}")?;
    }
    Ok(())
}
