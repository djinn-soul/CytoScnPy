use super::DeadCodeFixOptions;

use anyhow::Result;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

pub(super) fn collect_items_to_fix<'a>(
    results: &'a crate::analyzer::AnalysisResult,
    options: &DeadCodeFixOptions,
) -> HashMap<PathBuf, Vec<(&'static str, &'a crate::visitor::Definition)>> {
    let mut items_by_file: HashMap<PathBuf, Vec<(&'static str, &crate::visitor::Definition)>> =
        HashMap::new();

    if options.fix_functions {
        for def in &results.unused_functions {
            if def.confidence >= options.min_confidence {
                items_by_file
                    .entry((*def.file).clone())
                    .or_default()
                    .push(("function", def));
            }
        }
    }

    if options.fix_methods {
        for def in &results.unused_methods {
            if def.confidence >= options.min_confidence {
                items_by_file
                    .entry((*def.file).clone())
                    .or_default()
                    .push(("method", def));
            }
        }
    }

    if options.fix_classes {
        for def in &results.unused_classes {
            if def.confidence >= options.min_confidence {
                items_by_file
                    .entry((*def.file).clone())
                    .or_default()
                    .push(("class", def));
            }
        }
    }

    if options.fix_imports {
        for def in &results.unused_imports {
            if def.confidence >= options.min_confidence {
                items_by_file
                    .entry((*def.file).clone())
                    .or_default()
                    .push(("import", def));
            }
        }
    }

    if options.fix_variables {
        for def in &results.unused_variables {
            if def.confidence >= options.min_confidence {
                items_by_file
                    .entry((*def.file).clone())
                    .or_default()
                    .push(("variable", def));
            }
        }
    }

    items_by_file
}

pub(super) fn print_fix_stats<W: Write>(
    writer: &mut W,
    items_by_file: &HashMap<PathBuf, Vec<(&'static str, &crate::visitor::Definition)>>,
    results: &crate::analyzer::AnalysisResult,
    options: &DeadCodeFixOptions,
) -> Result<()> {
    if !options.verbose {
        return Ok(());
    }

    let total_items: usize = items_by_file.values().map(Vec::len).sum();
    let files_count = items_by_file.len();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for (item_type, _) in items_by_file.values().flatten() {
        *counts.entry(*item_type).or_default() += 1;
    }

    writeln!(writer, "[VERBOSE] Fix Statistics:")?;
    writeln!(writer, "   Files to modify: {files_count}")?;
    writeln!(writer, "   Items to remove: {total_items}")?;
    for (label, item_type) in [
        ("Functions", "function"),
        ("Methods", "method"),
        ("Classes", "class"),
        ("Imports", "import"),
        ("Variables", "variable"),
    ] {
        writeln!(
            writer,
            "   {label}: {}",
            counts.get(item_type).copied().unwrap_or_default()
        )?;
    }

    let total_skipped: usize = [
        results.unused_functions.as_slice(),
        results.unused_methods.as_slice(),
        results.unused_classes.as_slice(),
        results.unused_imports.as_slice(),
        results.unused_variables.as_slice(),
    ]
    .into_iter()
    .map(|definitions| {
        definitions
            .iter()
            .filter(|definition| definition.confidence < options.min_confidence)
            .count()
    })
    .sum();

    if total_skipped > 0 {
        writeln!(
            writer,
            "   Skipped (confidence < {}%): {}",
            options.min_confidence, total_skipped
        )?;
    }
    writeln!(writer)?;
    Ok(())
}
