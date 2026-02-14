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
    if options.verbose {
        let total_items: usize = items_by_file.values().map(Vec::len).sum();
        let files_count = items_by_file.len();

        let mut func_count = 0;
        let mut method_count = 0;
        let mut class_count = 0;
        let mut import_count = 0;
        let mut variable_count = 0;
        for items in items_by_file.values() {
            for (item_type, _) in items {
                match *item_type {
                    "function" => func_count += 1,
                    "method" => method_count += 1,
                    "class" => class_count += 1,
                    "import" => import_count += 1,
                    "variable" => variable_count += 1,
                    _ => {}
                }
            }
        }

        writeln!(writer, "[VERBOSE] Fix Statistics:")?;
        writeln!(writer, "   Files to modify: {files_count}")?;
        writeln!(writer, "   Items to remove: {total_items}")?;
        writeln!(writer, "   Functions: {func_count}")?;
        writeln!(writer, "   Methods: {method_count}")?;
        writeln!(writer, "   Classes: {class_count}")?;
        writeln!(writer, "   Imports: {import_count}")?;
        writeln!(writer, "   Variables: {variable_count}")?;

        let skipped_funcs = results
            .unused_functions
            .iter()
            .filter(|d| d.confidence < options.min_confidence)
            .count();
        let skipped_methods = results
            .unused_methods
            .iter()
            .filter(|d| d.confidence < options.min_confidence)
            .count();
        let skipped_classes = results
            .unused_classes
            .iter()
            .filter(|d| d.confidence < options.min_confidence)
            .count();
        let skipped_imports = results
            .unused_imports
            .iter()
            .filter(|d| d.confidence < options.min_confidence)
            .count();
        let skipped_variables = results
            .unused_variables
            .iter()
            .filter(|d| d.confidence < options.min_confidence)
            .count();
        let total_skipped =
            skipped_funcs + skipped_methods + skipped_classes + skipped_imports + skipped_variables;

        if total_skipped > 0 {
            writeln!(
                writer,
                "   Skipped (confidence < {}%): {}",
                options.min_confidence, total_skipped
            )?;
        }
        writeln!(writer)?;
    }
    Ok(())
}
