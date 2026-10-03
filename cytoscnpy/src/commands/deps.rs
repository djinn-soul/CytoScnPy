//! Dependency reporting preserves complete-scan status alongside findings.

mod environment;
mod findings;
use crate::deps::{analyze_dependencies, DependencyImportLocation, DepsOptions, DepsResult};
use anyhow::Result;
use colored::Colorize;
use environment::{write_extra_installed, write_orphan_installed, write_removable_branches};
use findings::{
    write_dev_dependency_in_production, write_missing_dependencies, write_stdlib_dependencies,
    write_transitive_dependencies, write_unused_dependencies,
};
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
        "scan_complete": result.scan_errors.is_empty(),
        "scan_errors": result.scan_errors,
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
    if !result.scan_errors.is_empty() {
        writeln!(
            writer,
            "\nDependency scan incomplete ({} errors):",
            result.scan_errors.len()
        )?;
        for error in &result.scan_errors {
            writeln!(writer, "  {}: {}", error.file.display(), error.error)?;
        }
    }
    write_summary(result, writer)?;
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
    result.scan_errors.is_empty()
        && result.unused.is_empty()
        && result.missing.is_empty()
        && result.transitive.is_empty()
        && result.dev_in_production.is_empty()
        && result.stdlib.is_empty()
        && result.extra_installed.is_empty()
        && result.orphan_installed.is_empty()
}
