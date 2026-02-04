use super::ranges::{find_def_range, find_import_edit, ImportEdit};
use super::{DeadCodeFixOptions, FixResult};
use crate::fix::{ByteRangeRewriter, Edit};

use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

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
        let mut class_count = 0;
        let mut import_count = 0;
        let mut variable_count = 0;
        for items in items_by_file.values() {
            for (item_type, _) in items {
                match *item_type {
                    "function" => func_count += 1,
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
        writeln!(writer, "   Classes: {class_count}")?;
        writeln!(writer, "   Imports: {import_count}")?;
        writeln!(writer, "   Variables: {variable_count}")?;

        let skipped_funcs = results
            .unused_functions
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
        let total_skipped = skipped_funcs + skipped_classes + skipped_imports + skipped_variables;

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

pub(super) fn apply_dead_code_fix_to_file<W: Write>(
    writer: &mut W,
    file_path: &Path,
    items: &[(&'static str, &crate::visitor::Definition)],
    options: &DeadCodeFixOptions,
) -> Result<Option<FixResult>> {
    #[cfg(feature = "cst")]
    use crate::cst::{AstCstMapper, CstParser};

    let file_path = crate::utils::validate_output_path(file_path, Some(&options.analysis_root))?;

    let content = match fs::read_to_string(&file_path) {
        Ok(c) => c,
        Err(e) => {
            writeln!(
                writer,
                "  {} {}: {}",
                "Skip:".yellow(),
                crate::utils::normalize_display_path(&file_path),
                e
            )?;
            return Ok(None);
        }
    };

    let parsed = match ruff_python_parser::parse_module(&content) {
        Ok(p) => p,
        Err(e) => {
            writeln!(
                writer,
                "  {} {}: {}",
                "Parse error:".red(),
                crate::utils::normalize_display_path(&file_path),
                e
            )?;
            return Ok(None);
        }
    };

    let module = parsed.into_syntax();
    let mut edits = Vec::new();
    let mut removed_names = Vec::new();

    #[cfg(feature = "cst")]
    let cst_mapper = if options.with_cst {
        CstParser::new()
            .ok()
            .and_then(|mut p| p.parse(&content).ok())
            .map(AstCstMapper::new)
    } else {
        None
    };

    for (item_type, def) in items {
        let mut edit_range = None;
        let mut replace_with = None;
        if *item_type == "variable" {
            if def.end_byte > def.start_byte {
                edit_range = Some((def.start_byte, def.end_byte));
                replace_with = Some("_");
            }
        } else if *item_type == "import" {
            match find_import_edit(&module.body, &def.simple_name, &content) {
                Some(ImportEdit::DeleteStmt(start, end)) => {
                    edit_range = Some((start, end));
                }
                Some(ImportEdit::DeleteAlias(start, end)) => {
                    edit_range = Some((start, end));
                }
                None => {}
            }
        } else {
            edit_range = find_def_range(&module.body, &def.simple_name, item_type);
        }

        if let Some((start, end)) = edit_range {
            let start_byte = start;
            let end_byte = end;

            #[cfg(feature = "cst")]
            let (start_byte, end_byte) = if let Some(mapper) = &cst_mapper {
                if *item_type == "function" || *item_type == "class" {
                    mapper.precise_range_for_def(start, end)
                } else {
                    (start_byte, end_byte)
                }
            } else {
                (start_byte, end_byte)
            };

            if options.dry_run {
                if replace_with.is_some() {
                    writeln!(
                        writer,
                        "  Would replace {} '{}' with '_' at {}:{}",
                        item_type,
                        def.simple_name,
                        crate::utils::normalize_display_path(&file_path),
                        def.line
                    )?;
                } else {
                    writeln!(
                        writer,
                        "  Would remove {} '{}' at {}:{}",
                        item_type,
                        def.simple_name,
                        crate::utils::normalize_display_path(&file_path),
                        def.line
                    )?;
                }
            } else {
                if let Some(replacement) = replace_with {
                    edits.push(Edit::new(start_byte, end_byte, replacement));
                } else {
                    edits.push(Edit::delete(start_byte, end_byte));
                }
                removed_names.push(def.simple_name.clone());
            }
        }
    }

    if !options.dry_run && !edits.is_empty() {
        // Sort edits: start ASC, then end DESC (longest touches first)
        edits.sort_by(|a, b| match a.start_byte.cmp(&b.start_byte) {
            std::cmp::Ordering::Equal => b.end_byte.cmp(&a.end_byte),
            other => other,
        });

        // Filter out edits contained in previous edits
        let mut filtered_edits = Vec::new();
        let mut last_end = 0;

        for edit in edits {
            if edit.start_byte >= last_end {
                // No overlap, keep it
                last_end = edit.end_byte;
                filtered_edits.push(edit);
            } else {
                // Overlap detected
                if edit.end_byte <= last_end {
                    // Fully contained in previous edit (e.g. unused variable inside unused function)
                    // Skip it safely as the outer edit removes it anyway.
                    continue;
                }
                // Partial overlap - we skip to avoid invalidating the previous edit or creating conflicts.
                // In valid AST analysis, partial overlap of nodes shouldn't happen for removals,
                // but we handle it safely here by dropping the conflicting edit.
            }
        }

        let mut rewriter = ByteRangeRewriter::new(content);
        rewriter.add_edits(filtered_edits);
        let fixed = match rewriter.apply() {
            Ok(fixed) => fixed,
            Err(e) => {
                writeln!(
                    writer,
                    "  {} {}: {}",
                    "Skip:".yellow(),
                    crate::utils::normalize_display_path(&file_path),
                    e
                )?;
                return Ok(None);
            }
        };

        if let Err(e) = ruff_python_parser::parse_module(&fixed) {
            writeln!(
                writer,
                "  {} {}: Produced invalid Python after fix: {}",
                "Skip:".yellow(),
                crate::utils::normalize_display_path(&file_path),
                e
            )?;
            return Ok(None);
        }

        let count = removed_names.len();
        fs::write(&file_path, fixed)?;
        writeln!(
            writer,
            "  {} {} ({} removed)",
            "Fixed:".green(),
            crate::utils::normalize_display_path(&file_path),
            count
        )?;
        return Ok(Some(FixResult {
            file: file_path.to_string_lossy().to_string(),
            items_removed: count,
            removed_names,
        }));
    }

    Ok(None)
}
