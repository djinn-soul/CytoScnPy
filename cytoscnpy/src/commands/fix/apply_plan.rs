use super::import_plan::plan_import_edits;
use super::names::DiscardNames;
use super::ranges::{find_def_range, find_method_edit};
use crate::fix::Edit;

use anyhow::Result;
use std::io::Write;
use std::path::Path;

pub(super) struct PlannedEdit {
    pub(super) start_byte: usize,
    pub(super) end_byte: usize,
    pub(super) replacement: Option<String>,
    pub(super) name: String,
    pub(super) removed_names: Vec<String>,
    pub(super) item_type: &'static str,
    pub(super) line: usize,
}

#[cfg(feature = "cst")]
pub(super) fn plan_edits(
    items: &[(&'static str, &crate::visitor::Definition)],
    module: &ruff_python_ast::ModModule,
    content: &str,
    cst_mapper: Option<&crate::cst::AstCstMapper>,
) -> Vec<PlannedEdit> {
    let mut planned = plan_import_edits(items, module, content);
    let mut names = DiscardNames::new(module);
    for (item_type, def) in items.iter().filter(|(kind, _)| *kind != "import") {
        if let Some(edit) = plan_item_edit(item_type, def, module, cst_mapper, &mut names) {
            planned.push(edit);
        }
    }
    planned
}

#[cfg(not(feature = "cst"))]
pub(super) fn plan_edits(
    items: &[(&'static str, &crate::visitor::Definition)],
    module: &ruff_python_ast::ModModule,
    content: &str,
) -> Vec<PlannedEdit> {
    let mut planned = plan_import_edits(items, module, content);
    let mut names = DiscardNames::new(module);
    for (item_type, def) in items.iter().filter(|(kind, _)| *kind != "import") {
        if let Some(edit) = plan_item_edit(item_type, def, module, &mut names) {
            planned.push(edit);
        }
    }
    planned
}

#[cfg(feature = "cst")]
fn plan_item_edit(
    item_type: &'static str,
    def: &crate::visitor::Definition,
    module: &ruff_python_ast::ModModule,
    cst_mapper: Option<&crate::cst::AstCstMapper>,
    names: &mut DiscardNames,
) -> Option<PlannedEdit> {
    let mut edit_range = None;
    let mut replacement: Option<String> = None;

    if item_type == "variable" {
        if def.end_byte > def.start_byte {
            edit_range = Some((def.start_byte, def.end_byte));
            replacement = Some(names.allocate());
        }
    } else if item_type == "method" {
        let edit = find_method_edit(&module.body, &def.simple_name, Some(def.start_byte));
        if let Some(edit) = edit {
            edit_range = Some((edit.start, edit.end));
            if edit.class_would_be_empty {
                replacement = Some("pass".to_owned());
            }
        }
    } else {
        edit_range = find_def_range(
            &module.body,
            &def.simple_name,
            item_type,
            Some(def.start_byte),
        );
    }

    let (start, end) = edit_range?;
    let (start, end) = if let Some(mapper) = cst_mapper {
        if item_type == "function" || item_type == "method" || item_type == "class" {
            mapper.precise_range_for_def(start, end)
        } else {
            (start, end)
        }
    } else {
        (start, end)
    };

    Some(PlannedEdit {
        start_byte: start,
        end_byte: end,
        replacement,
        name: def.simple_name.clone(),
        removed_names: vec![def.simple_name.clone()],
        item_type,
        line: def.line,
    })
}

#[cfg(not(feature = "cst"))]
fn plan_item_edit(
    item_type: &'static str,
    def: &crate::visitor::Definition,
    module: &ruff_python_ast::ModModule,
    names: &mut DiscardNames,
) -> Option<PlannedEdit> {
    let mut edit_range = None;
    let mut replacement: Option<String> = None;

    if item_type == "variable" {
        if def.end_byte > def.start_byte {
            edit_range = Some((def.start_byte, def.end_byte));
            replacement = Some(names.allocate());
        }
    } else if item_type == "method" {
        let edit = find_method_edit(&module.body, &def.simple_name, Some(def.start_byte));
        if let Some(edit) = edit {
            edit_range = Some((edit.start, edit.end));
            if edit.class_would_be_empty {
                replacement = Some("pass".to_owned());
            }
        }
    } else {
        edit_range = find_def_range(
            &module.body,
            &def.simple_name,
            item_type,
            Some(def.start_byte),
        );
    }

    let (start, end) = edit_range?;
    Some(PlannedEdit {
        start_byte: start,
        end_byte: end,
        replacement,
        name: def.simple_name.clone(),
        removed_names: vec![def.simple_name.clone()],
        item_type,
        line: def.line,
    })
}

pub(super) fn write_dry_run<W: Write>(
    writer: &mut W,
    file_path: &Path,
    planned: &[PlannedEdit],
) -> Result<()> {
    for item in planned {
        if item.replacement.is_some() {
            let replacement = item.replacement.as_deref().unwrap_or("_");
            writeln!(
                writer,
                "  Would replace {} '{}' with '{}' at {}:{}",
                item.item_type,
                item.name,
                replacement,
                crate::utils::normalize_display_path(file_path),
                item.line
            )?;
        } else {
            writeln!(
                writer,
                "  Would remove {} '{}' at {}:{}",
                item.item_type,
                item.name,
                crate::utils::normalize_display_path(file_path),
                item.line
            )?;
        }
    }
    Ok(())
}

pub(super) fn build_edits(planned: Vec<PlannedEdit>) -> (Vec<Edit>, Vec<String>) {
    let mut edits = Vec::with_capacity(planned.len());
    let mut removed_names = Vec::with_capacity(planned.len());

    for item in planned {
        if let Some(replacement) = item.replacement {
            edits.push(Edit::new(item.start_byte, item.end_byte, &replacement));
        } else {
            edits.push(Edit::delete(item.start_byte, item.end_byte));
        }
        removed_names.extend(item.removed_names);
    }

    (edits, removed_names)
}

/// Normalize edits before previewing or applying them, retaining covered definitions.
pub(super) fn normalize_planned_edits(mut planned: Vec<PlannedEdit>) -> Result<Vec<PlannedEdit>> {
    planned.sort_by_key(|item| (item.start_byte, std::cmp::Reverse(item.end_byte)));
    let mut normalized: Vec<PlannedEdit> = Vec::new();
    for item in planned {
        if let Some(previous) = normalized.last_mut() {
            if item.start_byte < previous.end_byte {
                if item.end_byte <= previous.end_byte && previous.replacement.is_none() {
                    // Deleting an enclosing definition also deletes definitions within it.
                    previous.removed_names.extend(item.removed_names);
                    continue;
                }
                anyhow::bail!(
                    "Conflicting fixes for '{}' and '{}' at byte ranges {}..{} and {}..{}",
                    previous.name,
                    item.name,
                    previous.start_byte,
                    previous.end_byte,
                    item.start_byte,
                    item.end_byte
                );
            }
        }
        normalized.push(item);
    }
    Ok(normalized)
}
