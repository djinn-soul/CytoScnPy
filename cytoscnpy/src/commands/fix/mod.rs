//! Dead code fix command.

mod apply;
mod apply_plan;
mod plan;
mod ranges;

#[cfg(test)]
mod tests;

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;

/// Options for dead code fix
#[derive(Debug, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct DeadCodeFixOptions {
    /// Minimum confidence threshold for auto-fix (0-100)
    pub min_confidence: u8,
    /// Dry-run mode (show what would change)
    pub dry_run: bool,
    /// Emit dry-run output as JSON plan
    pub json_output: bool,
    /// Fix functions
    pub fix_functions: bool,
    /// Fix methods
    pub fix_methods: bool,
    /// Fix classes
    pub fix_classes: bool,
    /// Fix imports
    pub fix_imports: bool,
    /// Fix unused variables (renames to `_`)
    pub fix_variables: bool,
    /// Verbose output
    pub verbose: bool,
    /// Use CST for precise fixing
    pub with_cst: bool,
    /// Analysis root for path containment
    pub analysis_root: PathBuf,
}

/// Result of dead code fix operation
#[derive(Debug, Serialize)]
pub struct FixResult {
    /// File that was fixed
    pub file: String,
    /// Number of items removed
    pub items_removed: usize,
    /// Number of lines removed
    pub lines_removed: usize,
    /// Names of removed items
    pub removed_names: Vec<String>,
    /// Planned edits (only present in dry-run JSON mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_edits: Option<Vec<FixPlanItem>>,
}

/// A planned fix operation for dry-run JSON output.
#[derive(Debug, Serialize, Clone)]
pub struct FixPlanItem {
    /// Stable identifier for deterministic diffing across runs.
    pub stable_id: String,
    /// Item type being changed (`function`, `method`, `class`, `import`, `variable`)
    pub item_type: String,
    /// Symbol name
    pub name: String,
    /// Source line
    pub line: usize,
    /// Start byte offset (inclusive)
    pub start_byte: usize,
    /// End byte offset (exclusive)
    pub end_byte: usize,
    /// Replacement text, if any (`_` / `pass`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
}

/// Apply --fix to dead code findings.
///
/// # Errors
///
/// Returns an error if file I/O fails or fix fails.
#[allow(clippy::too_many_lines)]
pub fn run_fix_deadcode<W: Write>(
    results: &crate::analyzer::AnalysisResult,
    options: &DeadCodeFixOptions,
    mut writer: W,
) -> Result<Vec<FixResult>> {
    if options.dry_run && !options.json_output {
        writeln!(
            writer,
            "\n{}",
            "[DRY-RUN] Dead code that would be removed:".yellow()
        )?;
    } else if !options.dry_run && !options.json_output {
        writeln!(writer, "\n{}", "Applying dead code fixes...".cyan())?;
    }

    let items_by_file = plan::collect_items_to_fix(results, options);

    if items_by_file.is_empty() {
        if options.json_output {
            write_json_fix_payload(&mut writer, options, &[])?;
        } else {
            writeln!(
                writer,
                "  No items with confidence >= {} to fix.",
                options.min_confidence
            )?;
        }
        return Ok(vec![]);
    }

    if !options.json_output {
        plan::print_fix_stats(&mut writer, &items_by_file, results, options)?;
    }

    let mut work_items: Vec<_> = items_by_file.into_iter().collect();
    work_items.sort_by(|(path_a, _), (path_b, _)| {
        crate::utils::normalize_display_path(path_a)
            .cmp(&crate::utils::normalize_display_path(path_b))
    });

    let mut all_results = Vec::new();

    for (file_path, mut items) in work_items {
        items.sort_by(|(type_a, def_a), (type_b, def_b)| {
            def_a
                .line
                .cmp(&def_b.line)
                .then_with(|| def_a.start_byte.cmp(&def_b.start_byte))
                .then_with(|| type_a.cmp(type_b))
                .then_with(|| def_a.simple_name.cmp(&def_b.simple_name))
        });
        let res = if options.json_output {
            let mut quiet = Vec::new();
            apply::apply_dead_code_fix_to_file(&mut quiet, &file_path, &items, options)?
        } else {
            apply::apply_dead_code_fix_to_file(&mut writer, &file_path, &items, options)?
        };
        if let Some(res) = res {
            all_results.push(res);
        }
    }

    if options.json_output {
        write_json_fix_payload(&mut writer, options, &all_results)?;
        return Ok(all_results);
    }

    if !all_results.is_empty() && !options.dry_run && !options.json_output {
        let total_items_removed: usize = all_results.iter().map(|r| r.items_removed).sum();
        let total_lines_removed: usize = all_results.iter().map(|r| r.lines_removed).sum();

        let total_targeted = total_targeted_items(results, options);

        let items_pct = if total_targeted > 0 {
            (total_items_removed as f64 / total_targeted as f64) * 100.0
        } else {
            0.0
        };

        let lines_pct = if results.analysis_summary.total_lines_analyzed > 0 {
            (total_lines_removed as f64 / results.analysis_summary.total_lines_analyzed as f64)
                * 100.0
        } else {
            0.0
        };

        writeln!(writer, "\n{}", "Fix Summary:".green().bold())?;
        writeln!(
            writer,
            "  Total items fixed: {total_items_removed}/{total_targeted} ({items_pct:.1}%)"
        )?;
        writeln!(
            writer,
            "  Total lines removed: {total_lines_removed}/{} ({lines_pct:.2}%)",
            results.analysis_summary.total_lines_analyzed
        )?;
    }

    Ok(all_results)
}

fn total_targeted_items(
    results: &crate::analyzer::AnalysisResult,
    options: &DeadCodeFixOptions,
) -> usize {
    let mut total = 0usize;
    if options.fix_functions {
        total += results.unused_functions.len();
    }
    if options.fix_methods {
        total += results.unused_methods.len();
    }
    if options.fix_classes {
        total += results.unused_classes.len();
    }
    if options.fix_imports {
        total += results.unused_imports.len();
    }
    if options.fix_variables {
        total += results.unused_variables.len();
    }
    total
}

fn write_json_fix_payload<W: Write>(
    writer: &mut W,
    options: &DeadCodeFixOptions,
    all_results: &[FixResult],
) -> Result<()> {
    if options.dry_run {
        let planned_items: usize = all_results.iter().map(|result| result.items_removed).sum();
        let payload = serde_json::json!({
            "schema_version": "2",
            "kind": "dead_code_fix_plan",
            "min_confidence": options.min_confidence,
            "planned_files": all_results.len(),
            "planned_items": planned_items,
            "plans": all_results,
        });
        serde_json::to_writer_pretty(&mut *writer, &payload)?;
        writeln!(writer)?;
        return Ok(());
    }

    let items_removed: usize = all_results.iter().map(|result| result.items_removed).sum();
    let lines_removed: usize = all_results.iter().map(|result| result.lines_removed).sum();
    let payload = serde_json::json!({
        "schema_version": "2",
        "kind": "dead_code_fix_report",
        "min_confidence": options.min_confidence,
        "applied_files": all_results.len(),
        "items_removed": items_removed,
        "lines_removed": lines_removed,
        "results": all_results,
    });
    serde_json::to_writer_pretty(&mut *writer, &payload)?;
    writeln!(writer)?;
    Ok(())
}
