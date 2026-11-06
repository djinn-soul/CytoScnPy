use super::apply_plan::{build_edits, write_dry_run};
use super::verified::{FixSource, VerifiedFix};
use super::{DeadCodeFixOptions, FixPlanItem, FixResult};
use crate::ipynb::NotebookSource;

use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::io::Write;
use std::path::Path;

pub(super) fn apply_dead_code_fix_to_file<W: Write>(
    writer: &mut W,
    file_path: &Path,
    items: &[(&'static str, &crate::visitor::Definition)],
    options: &DeadCodeFixOptions,
) -> Result<Option<FixResult>> {
    let file_path = crate::utils::validate_source_path(file_path, Some(&options.analysis_root))?;

    // Notebook findings index the joined code cells, not the JSON on disk.
    let notebook = file_path
        .extension()
        .is_some_and(|ext| ext == "ipynb")
        .then(|| NotebookSource::read(&file_path, Some(&options.analysis_root)))
        .transpose()
        .with_context(|| format!("Failed to read fix source {}", file_path.display()))?;
    let content = match &notebook {
        Some(notebook) => notebook.code.clone(),
        None => fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read fix source {}", file_path.display()))?,
    };
    let source = FixSource::parse(&content, options.with_cst)
        .with_context(|| format!("Failed to parse fix source {}", file_path.display()))?;
    let VerifiedFix { planned, fixed } = source.plan(&file_path, items)?;
    let fixed_cells = notebook
        .as_ref()
        .map(|notebook| {
            notebook.apply_edits(planned.iter().map(|edit| {
                (
                    edit.start_byte,
                    edit.end_byte,
                    edit.replacement.as_deref().unwrap_or(""),
                )
            }))
        })
        .transpose()
        .with_context(|| format!("Failed to fix {}", file_path.display()))?;

    if options.dry_run {
        if options.json_output {
            let removed_names: Vec<_> = planned
                .iter()
                .flat_map(|item| item.removed_names.iter().cloned())
                .collect();
            // `--ipynb-cells` has already made definition lines cell-relative.
            let lines_in_cells = items.iter().any(|(_, def)| def.cell_number.is_some());
            let planned_edits = planned
                .iter()
                .map(|item| {
                    // Notebook edits are reported relative to their cell: the
                    // offsets, the line and the stable ID alike.
                    let cell = notebook.as_ref().and_then(|notebook| {
                        notebook.cell_containing(item.start_byte, item.end_byte)
                    });
                    let start_byte = item.start_byte - cell.map_or(0, |cell| cell.start);
                    let end_byte = item.end_byte - cell.map_or(0, |cell| cell.start);
                    let (line, location) = match cell {
                        Some(cell) if lines_in_cells => {
                            (item.line, format!("[cell {}]", cell.index))
                        }
                        Some(cell) => (
                            cell.relative_line(item.line),
                            format!("[cell {}]", cell.index),
                        ),
                        None => (item.line, String::new()),
                    };
                    FixPlanItem {
                        stable_id: format!(
                            "{}:{}{location}:{start_byte}:{end_byte}",
                            item.item_type, item.name
                        ),
                        item_type: item.item_type.to_owned(),
                        name: item.name.clone(),
                        line,
                        cell_number: cell.map(|cell| cell.index),
                        start_byte,
                        end_byte,
                        replacement: item.replacement.clone(),
                    }
                })
                .collect::<Vec<_>>();
            return Ok(Some(FixResult {
                file: file_path.to_string_lossy().to_string(),
                items_removed: removed_names.len(),
                lines_removed: 0,
                removed_names,
                planned_edits: Some(planned_edits),
            }));
        }
        write_dry_run(writer, &file_path, &planned)?;
        return Ok(None);
    }

    let original_line_count = content.lines().count();
    let (_, removed_names) = build_edits(planned);

    let fixed_line_count = fixed.lines().count();
    let lines_removed = original_line_count.saturating_sub(fixed_line_count);

    let count = removed_names.len();
    match &fixed_cells {
        Some(cells) => {
            crate::ipynb::write_cell_sources(&file_path, Some(&options.analysis_root), cells)?;
        }
        None => fs::write(&file_path, fixed)?,
    }
    writeln!(
        writer,
        "  {} {} ({} removed, {} lines)",
        "Fixed:".green(),
        crate::utils::normalize_display_path(&file_path),
        count,
        lines_removed
    )?;
    Ok(Some(FixResult {
        file: file_path.to_string_lossy().to_string(),
        items_removed: count,
        lines_removed,
        removed_names,
        planned_edits: None,
    }))
}
