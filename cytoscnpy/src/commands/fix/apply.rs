use super::apply_plan::{build_edits, plan_edits, write_dry_run};
use super::{DeadCodeFixOptions, FixResult};
use crate::fix::{ByteRangeRewriter, Edit};

use anyhow::Result;
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

    let Some(content) = read_source_or_report(writer, &file_path)? else {
        return Ok(None);
    };
    let Some(module) = parse_module_or_report(writer, &file_path, &content)? else {
        return Ok(None);
    };

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

    if planned.is_empty() {
        return Ok(None);
    }

    if options.dry_run {
        write_dry_run(writer, &file_path, &planned)?;
        return Ok(None);
    }

    let (edits, removed_names) = build_edits(planned);
    let Some(fixed) = apply_edits(writer, &file_path, content, edits)? else {
        return Ok(None);
    };

    if !validate_fixed_source(writer, &file_path, &fixed)? {
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
    Ok(Some(FixResult {
        file: file_path.to_string_lossy().to_string(),
        items_removed: count,
        removed_names,
    }))
}

fn read_source_or_report<W: Write>(writer: &mut W, file_path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(file_path) {
        Ok(content) => Ok(Some(content)),
        Err(e) => {
            writeln!(
                writer,
                "  {} {}: {}",
                "Skip:".yellow(),
                crate::utils::normalize_display_path(file_path),
                e
            )?;
            Ok(None)
        }
    }
}

fn parse_module_or_report<W: Write>(
    writer: &mut W,
    file_path: &Path,
    content: &str,
) -> Result<Option<ruff_python_ast::ModModule>> {
    match ruff_python_parser::parse_module(content) {
        Ok(parsed) => Ok(Some(parsed.into_syntax())),
        Err(e) => {
            writeln!(
                writer,
                "  {} {}: {}",
                "Parse error:".red(),
                crate::utils::normalize_display_path(file_path),
                e
            )?;
            Ok(None)
        }
    }
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

fn apply_edits<W: Write>(
    writer: &mut W,
    file_path: &Path,
    content: String,
    edits: Vec<Edit>,
) -> Result<Option<String>> {
    if edits.is_empty() {
        return Ok(None);
    }

    let filtered = filter_overlapping_edits(edits);
    let mut rewriter = ByteRangeRewriter::new(content);
    rewriter.add_edits(filtered);
    match rewriter.apply() {
        Ok(fixed) => Ok(Some(fixed)),
        Err(e) => {
            writeln!(
                writer,
                "  {} {}: {}",
                "Skip:".yellow(),
                crate::utils::normalize_display_path(file_path),
                e
            )?;
            Ok(None)
        }
    }
}

fn filter_overlapping_edits(mut edits: Vec<Edit>) -> Vec<Edit> {
    edits.sort_by(|a, b| match a.start_byte.cmp(&b.start_byte) {
        std::cmp::Ordering::Equal => b.end_byte.cmp(&a.end_byte),
        other => other,
    });

    let mut filtered = Vec::new();
    let mut last_end = 0;

    for edit in edits {
        if edit.start_byte >= last_end {
            last_end = edit.end_byte;
            filtered.push(edit);
        } else if edit.end_byte <= last_end {
            // Fully contained in previous edit - safe to skip.
        } else {
            // Partial overlap - skip to avoid conflicting edits.
        }
    }

    filtered
}

fn validate_fixed_source<W: Write>(writer: &mut W, file_path: &Path, fixed: &str) -> Result<bool> {
    if let Err(e) = ruff_python_parser::parse_module(fixed) {
        writeln!(
            writer,
            "  {} {}: Produced invalid Python after fix: {}",
            "Skip:".yellow(),
            crate::utils::normalize_display_path(file_path),
            e
        )?;
        return Ok(false);
    }

    Ok(true)
}
