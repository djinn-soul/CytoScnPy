use crate::deps::{
    analyze_dependencies, DeclaredDependency, DependencyImportLocation, DepsOptions, DepsResult,
    MissingDependency,
};
use crate::rules::ids::{
    RULE_ID_DEV_DEPENDENCY_IN_PROD, RULE_ID_MISSING_DEPENDENCY, RULE_ID_STDLIB_DEPENDENCY,
    RULE_ID_TRANSITIVE_DEPENDENCY, RULE_ID_UNUSED_DEPENDENCY,
};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Cell, Color, Table};
use serde_json::json;
use std::io::Write;

/// Executes the deps subcommand — v3 edition.
/// Reports unused, missing, extra-installed, orphan, and removable-branch findings.
pub fn run_deps<W: std::io::Write>(
    options: &DepsOptions<'_>,
    writer: &mut W,
) -> Result<crate::deps::DepsResult> {
    let result = analyze_dependencies(options);

    if options.json {
        write_json_deps(&result, writer)?;
    } else {
        write_text_deps(&result, writer)?;
    }

    Ok(result)
}

fn write_json_deps<W: Write>(result: &DepsResult, writer: &mut W) -> Result<()> {
    let out = json!({
        "unused": result.unused.iter().map(|d| d.package_name.clone()).collect::<Vec<_>>(),
        "missing": result.missing,
        "missing_details": result.missing_details.iter().map(|d| json!({
            "import_name": d.import_name,
            "locations": dependency_locations_json(&d.locations),
        })).collect::<Vec<_>>(),
        "transitive": result.transitive.iter().map(|d| json!({
            "import_name": d.import_name,
            "package_name": d.package_name,
            "locations": dependency_locations_json(&d.locations),
        })).collect::<Vec<_>>(),
        "dev_in_production": result.dev_in_production.iter().map(|d| json!({
            "import_name": d.import_name,
            "package_name": d.dependency.package_name,
            "locations": dependency_locations_json(&d.locations),
        })).collect::<Vec<_>>(),
        "stdlib": result.stdlib.iter().map(|d| d.package_name.clone()).collect::<Vec<_>>(),
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
    Ok(())
}

fn dependency_locations_json(locations: &[DependencyImportLocation]) -> Vec<serde_json::Value> {
    locations
        .iter()
        .map(|location| {
            json!({
                "file": location.file,
                "line": location.line,
                "column": location.column,
            })
        })
        .collect()
}

fn write_text_deps<W: Write>(result: &DepsResult, writer: &mut W) -> Result<()> {
    write_unused_dependencies(&result.unused, writer)?;
    write_missing_dependencies(&result.missing_details, writer)?;
    write_transitive_dependencies(result, writer)?;
    write_dev_dependency_in_production(result, writer)?;
    write_stdlib_dependencies(result, writer)?;
    write_extra_installed(result, writer)?;
    write_orphan_installed(result, writer)?;
    write_removable_branches(result, writer)?;
    write_summary(result, writer)?;
    Ok(())
}

fn write_unused_dependencies<W: Write>(
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

fn write_missing_dependencies<W: Write>(
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

fn write_transitive_dependencies<W: Write>(result: &DepsResult, writer: &mut W) -> Result<()> {
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

fn write_dev_dependency_in_production<W: Write>(result: &DepsResult, writer: &mut W) -> Result<()> {
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

fn write_stdlib_dependencies<W: Write>(result: &DepsResult, writer: &mut W) -> Result<()> {
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

fn write_extra_installed<W: Write>(result: &DepsResult, writer: &mut W) -> Result<()> {
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

fn write_orphan_installed<W: Write>(result: &DepsResult, writer: &mut W) -> Result<()> {
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

fn write_removable_branches<W: Write>(result: &DepsResult, writer: &mut W) -> Result<()> {
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

fn write_summary<W: Write>(result: &DepsResult, writer: &mut W) -> Result<()> {
    if deps_are_clean(result) {
        writeln!(
            writer,
            "{}",
            "No unused, missing, extra, or orphan dependencies found!".green()
        )?;
    } else {
        writeln!(
            writer,
            "\nFound: {} unused, {} missing, {} transitive, {} dev-in-prod, {} stdlib, {} extra installed, {} orphan.",
            result.unused.len(),
            result.missing.len(),
            result.transitive.len(),
            result.dev_in_production.len(),
            result.stdlib.len(),
            result.extra_installed.len(),
            result.orphan_installed.len(),
        )?;
    }
    Ok(())
}

fn deps_are_clean(result: &DepsResult) -> bool {
    result.unused.is_empty()
        && result.missing.is_empty()
        && result.transitive.is_empty()
        && result.dev_in_production.is_empty()
        && result.stdlib.is_empty()
        && result.extra_installed.is_empty()
        && result.orphan_installed.is_empty()
}
