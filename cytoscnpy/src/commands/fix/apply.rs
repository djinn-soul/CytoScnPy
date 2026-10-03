use super::apply_plan::{build_edits, normalize_planned_edits, plan_edits, write_dry_run};
use super::{syntax::preserve_syntax, DeadCodeFixOptions, FixPlanItem, FixResult};
use crate::fix::{ByteRangeRewriter, Edit};

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
    let file_path = crate::utils::validate_output_path(file_path, Some(&options.analysis_root))?;

    let content = fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read fix source {}", file_path.display()))?;
    let module = ruff_python_parser::parse_module(&content)
        .with_context(|| format!("Failed to parse fix source {}", file_path.display()))?
        .into_syntax();

    #[cfg(feature = "cst")]
    let cst_mapper = build_cst_mapper(&content, options);

    let planned = {
        #[cfg(feature = "cst")]
        {
            plan_edits(items, &module, &content, cst_mapper.as_ref())
        }
        #[cfg(not(feature = "cst"))]
        {
            plan_edits(items, &module, &content)
        }
    };

    let mut planned = normalize_planned_edits(planned)
        .with_context(|| format!("Failed to plan fixes for {}", file_path.display()))?;
    anyhow::ensure!(
        !planned.is_empty(),
        "No fixes could be planned for selected definitions in {}",
        file_path.display()
    );
    anyhow::ensure!(
        planned
            .iter()
            .map(|edit| edit.removed_names.len())
            .sum::<usize>()
            == items.len(),
        "Some selected definitions could not be located in {}",
        file_path.display()
    );
    preserve_syntax(&module, &content, &mut planned);
    let planned = normalize_planned_edits(planned)
        .with_context(|| format!("Failed to preserve syntax for {}", file_path.display()))?;
    let mut preview = ByteRangeRewriter::new(&content);
    preview.add_edits(planned.iter().map(|edit| {
        Edit::new(
            edit.start_byte,
            edit.end_byte,
            edit.replacement.as_deref().unwrap_or(""),
        )
    }));
    let fixed = preview.apply_verified().with_context(|| {
        format!(
            "Failed to fix {}: generated invalid Python",
            file_path.display()
        )
    })?;

    if options.dry_run {
        if options.json_output {
            let removed_names: Vec<_> = planned
                .iter()
                .flat_map(|item| item.removed_names.iter().cloned())
                .collect();
            let planned_edits = planned
                .iter()
                .map(|item| FixPlanItem {
                    stable_id: format!(
                        "{}:{}:{}:{}",
                        item.item_type, item.name, item.start_byte, item.end_byte
                    ),
                    item_type: item.item_type.to_owned(),
                    name: item.name.clone(),
                    line: item.line,
                    start_byte: item.start_byte,
                    end_byte: item.end_byte,
                    replacement: item.replacement.clone(),
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
    fs::write(&file_path, fixed)?;
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

#[cfg(feature = "cst")]
fn build_cst_mapper(
    content: &str,
    options: &DeadCodeFixOptions,
) -> Option<crate::cst::AstCstMapper> {
    use crate::cst::CstParser;

    if !options.with_cst {
        return None;
    }

    CstParser::new()
        .ok()
        .and_then(|mut parser| parser.parse(content).ok())
        .map(crate::cst::AstCstMapper::new)
}
