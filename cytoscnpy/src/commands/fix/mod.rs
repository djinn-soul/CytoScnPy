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
    /// Fix functions
    pub fix_functions: bool,
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
    if options.dry_run {
        writeln!(
            writer,
            "\n{}",
            "[DRY-RUN] Dead code that would be removed:".yellow()
        )?;
    } else {
        writeln!(writer, "\n{}", "Applying dead code fixes...".cyan())?;
    }

    let items_by_file = plan::collect_items_to_fix(results, options);

    if items_by_file.is_empty() {
        writeln!(
            writer,
            "  No items with confidence >= {} to fix.",
            options.min_confidence
        )?;
        return Ok(vec![]);
    }

    plan::print_fix_stats(&mut writer, &items_by_file, results, options)?;

    let mut all_results = Vec::new();

    for (file_path, items) in items_by_file {
        if let Some(res) =
            apply::apply_dead_code_fix_to_file(&mut writer, &file_path, &items, options)?
        {
            all_results.push(res);
        }
    }

    if !all_results.is_empty() {
        let total_items_removed: usize = all_results.iter().map(|r| r.items_removed).sum();
        let total_lines_removed: usize = all_results.iter().map(|r| r.lines_removed).sum();

        let total_dead_code = results.unused_functions.len()
            + results.unused_methods.len()
            + results.unused_imports.len()
            + results.unused_classes.len()
            + results.unused_variables.len();

        let items_pct = if total_dead_code > 0 {
            (total_items_removed as f64 / total_dead_code as f64) * 100.0
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
            "  Total items fixed: {total_items_removed}/{total_dead_code} ({items_pct:.1}%)"
        )?;
        writeln!(
            writer,
            "  Total lines removed: {total_lines_removed}/{} ({lines_pct:.2}%)",
            results.analysis_summary.total_lines_analyzed
        )?;
    }

    Ok(all_results)
}
