use crate::deps::{DeclaredDependency, DependencyImportLocation, DepsResult, MissingDependency};
use crate::rules::ids::{
    RULE_ID_DEV_DEPENDENCY_IN_PROD, RULE_ID_MISSING_DEPENDENCY, RULE_ID_STDLIB_DEPENDENCY,
    RULE_ID_TRANSITIVE_DEPENDENCY, RULE_ID_UNUSED_DEPENDENCY,
};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Cell, Color, Table};
use std::io::Write;

pub(super) fn write_unused_dependencies<W: Write>(
    unused: &[DeclaredDependency],
    writer: &mut W,
) -> Result<()> {
    if unused.is_empty() {
        return Ok(());
    }

    writeln!(
        writer,
        "\n{}",
        format!("Unused Dependencies ({RULE_ID_UNUSED_DEPENDENCY})")
            .red()
            .bold()
    )?;
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec!["Package Name", "Declared In", "Type"]);

    for dep in unused {
        table.add_row(vec![
            Cell::new(&dep.package_name).fg(Color::Yellow),
            Cell::new(dependency_source_name(dep)),
            Cell::new(dependency_kind(dep)),
        ]);
    }
    writeln!(writer, "{table}")?;
    Ok(())
}

fn dependency_source_name(dep: &DeclaredDependency) -> String {
    match &dep.source {
        crate::deps::DependencySource::Pyproject => "pyproject.toml".to_owned(),
        crate::deps::DependencySource::Requirements(file)
        | crate::deps::DependencySource::Setup(file) => file.clone(),
    }
}

fn dependency_kind(dep: &DeclaredDependency) -> &'static str {
    if dep.is_dev {
        "dev"
    } else {
        "prod"
    }
}

fn first_location(locations: &[DependencyImportLocation]) -> String {
    locations.first().map_or_else(
        || "-".to_owned(),
        |location| format!("{}:{}", location.file.display(), location.line),
    )
}

pub(super) fn write_missing_dependencies<W: Write>(
    missing: &[MissingDependency],
    writer: &mut W,
) -> Result<()> {
    if missing.is_empty() {
        return Ok(());
    }

    writeln!(
        writer,
        "\n{}",
        format!("Missing Dependencies ({RULE_ID_MISSING_DEPENDENCY})")
            .red()
            .bold()
    )?;
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec!["Import Name", "Location"]);

    for missing in missing {
        table.add_row(vec![
            Cell::new(&missing.import_name).fg(Color::Yellow),
            Cell::new(first_location(&missing.locations)),
        ]);
    }
    writeln!(writer, "{table}")?;
    Ok(())
}

pub(super) fn write_transitive_dependencies<W: Write>(
    result: &DepsResult,
    writer: &mut W,
) -> Result<()> {
    if result.transitive.is_empty() {
        return Ok(());
    }

    writeln!(
        writer,
        "\n{}",
        format!("Transitive Dependencies ({RULE_ID_TRANSITIVE_DEPENDENCY})")
            .red()
            .bold()
    )?;
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec!["Import Name", "Package Name", "Location"]);

    for dep in &result.transitive {
        table.add_row(vec![
            Cell::new(&dep.import_name).fg(Color::Yellow),
            Cell::new(&dep.package_name),
            Cell::new(first_location(&dep.locations)),
        ]);
    }
    writeln!(writer, "{table}")?;
    Ok(())
}

pub(super) fn write_dev_dependency_in_production<W: Write>(
    result: &DepsResult,
    writer: &mut W,
) -> Result<()> {
    if result.dev_in_production.is_empty() {
        return Ok(());
    }

    writeln!(
        writer,
        "\n{}",
        format!("Development Dependency Used in Production ({RULE_ID_DEV_DEPENDENCY_IN_PROD})")
            .red()
            .bold()
    )?;
    let mut table = Table::new();
    table.load_preset(UTF8_FULL).set_header(vec![
        "Import Name",
        "Package Name",
        "Declared In",
        "Location",
    ]);

    for dep in &result.dev_in_production {
        table.add_row(vec![
            Cell::new(&dep.import_name).fg(Color::Yellow),
            Cell::new(&dep.dependency.package_name),
            Cell::new(dependency_source_name(&dep.dependency)),
            Cell::new(first_location(&dep.locations)),
        ]);
    }
    writeln!(writer, "{table}")?;
    Ok(())
}

pub(super) fn write_stdlib_dependencies<W: Write>(
    result: &DepsResult,
    writer: &mut W,
) -> Result<()> {
    if result.stdlib.is_empty() {
        return Ok(());
    }

    writeln!(
        writer,
        "\n{}",
        format!("Standard Library Dependencies ({RULE_ID_STDLIB_DEPENDENCY})")
            .red()
            .bold()
    )?;
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec!["Package Name", "Declared In", "Type"]);

    for dep in &result.stdlib {
        table.add_row(vec![
            Cell::new(&dep.package_name).fg(Color::Yellow),
            Cell::new(dependency_source_name(dep)),
            Cell::new(dependency_kind(dep)),
        ]);
    }
    writeln!(writer, "{table}")?;
    Ok(())
}
