use super::ranges::{find_def_range, find_import_edit, find_method_edit, ImportEdit};
use crate::fix::Edit;

use anyhow::Result;
use std::io::Write;
use std::path::Path;

pub(super) struct PlannedEdit {
    pub(super) start_byte: usize,
    pub(super) end_byte: usize,
    pub(super) replacement: Option<String>,
    pub(super) name: String,
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
    let mut planned = Vec::new();
    for (item_type, def) in items {
        if let Some(edit) = plan_item_edit(item_type, def, module, content, cst_mapper) {
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
    let mut planned = Vec::new();
    for (item_type, def) in items {
        if let Some(edit) = plan_item_edit(item_type, def, module, content) {
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
    content: &str,
    cst_mapper: Option<&crate::cst::AstCstMapper>,
) -> Option<PlannedEdit> {
    let mut edit_range = None;
    let mut replacement: Option<String> = None;

    if item_type == "variable" {
        if def.end_byte > def.start_byte {
            edit_range = Some((def.start_byte, def.end_byte));
            replacement = Some("_".to_owned());
        }
    } else if item_type == "method" {
        if let Some(edit) = find_method_edit(&module.body, &def.simple_name) {
            edit_range = Some((edit.start, edit.end));
            if edit.class_would_be_empty {
                replacement = Some("pass".to_owned());
            }
        }
    } else if item_type == "import" {
        if let Some(edit) = find_import_edit(&module.body, &def.simple_name, content) {
            match edit {
                ImportEdit::DeleteStmt(start, end) | ImportEdit::DeleteAlias(start, end) => {
                    edit_range = Some((start, end));
                }
            }
        }
    } else {
        edit_range = find_def_range(&module.body, &def.simple_name, item_type);
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
        item_type,
        line: def.line,
    })
}

#[cfg(not(feature = "cst"))]
fn plan_item_edit(
    item_type: &'static str,
    def: &crate::visitor::Definition,
    module: &ruff_python_ast::ModModule,
    content: &str,
) -> Option<PlannedEdit> {
    let mut edit_range = None;
    let mut replacement: Option<String> = None;

    if item_type == "variable" {
        if def.end_byte > def.start_byte {
            edit_range = Some((def.start_byte, def.end_byte));
            replacement = Some("_".to_owned());
        }
    } else if item_type == "method" {
        if let Some(edit) = find_method_edit(&module.body, &def.simple_name) {
            edit_range = Some((edit.start, edit.end));
            if edit.class_would_be_empty {
                replacement = Some("pass".to_owned());
            }
        }
    } else if item_type == "import" {
        if let Some(edit) = find_import_edit(&module.body, &def.simple_name, content) {
            match edit {
                ImportEdit::DeleteStmt(start, end) | ImportEdit::DeleteAlias(start, end) => {
                    edit_range = Some((start, end));
                }
            }
        }
    } else {
        edit_range = find_def_range(&module.body, &def.simple_name, item_type);
    }

    let (start, end) = edit_range?;
    Some(PlannedEdit {
        start_byte: start,
        end_byte: end,
        replacement,
        name: def.simple_name.clone(),
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
            writeln!(
                writer,
                "  Would replace {} '{}' with '_' at {}:{}",
                item.item_type,
                item.name,
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
        removed_names.push(item.name);
    }

    (edits, removed_names)
}
