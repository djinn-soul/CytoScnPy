//! Terminal and JSON reporters for mutable global state analysis.

use anyhow::Result;
use std::io::Write;
use std::path::Path;

use super::types::{GlobalKind, GlobalsResult};

/// Maximum number of matches displayed per kind in terminal output.
const MAX_DISPLAYED_PER_KIND: usize = 30;

/// Emits a human-readable terminal report to `writer`.
pub fn print_terminal_report<W: Write>(
    result: &GlobalsResult,
    base: Option<&Path>,
    writer: &mut W,
) -> Result<()> {
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;
    writeln!(writer, "  Mutable Global State Analysis")?;
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;

    if result.is_clean() {
        writeln!(
            writer,
            "  ✓ No mutable global state detected across {} files.",
            result.files_scanned
        )?;
        writeln!(
            writer,
            "═══════════════════════════════════════════════════════════"
        )?;
        return Ok(());
    }

    let s = &result.stats;
    writeln!(writer, "  Files scanned     : {}", result.files_scanned)?;
    writeln!(writer, "  Total globals     : {}", s.total_globals)?;
    writeln!(writer, "  Affected files    : {}", s.affected_files)?;
    writeln!(
        writer,
        "───────────────────────────────────────────────────────────"
    )?;
    writeln!(writer, "  {:35} {:>6}", "Kind", "Count")?;
    writeln!(
        writer,
        "  {:35} {:>6}",
        "───────────────────────────────────", "─────"
    )?;

    if s.module_collection_count > 0 {
        writeln!(
            writer,
            "  {:35} {:>6}",
            "Module Collections (Python)", s.module_collection_count
        )?;
    }
    if s.class_variable_count > 0 {
        writeln!(
            writer,
            "  {:35} {:>6}",
            "Class Variables (Python)", s.class_variable_count
        )?;
    }
    if s.global_mutation_count > 0 {
        writeln!(
            writer,
            "  {:35} {:>6}",
            "Global Mutations (Python)", s.global_mutation_count
        )?;
    }
    if s.rust_static_mut_count > 0 {
        writeln!(
            writer,
            "  {:35} {:>6}",
            "Rust `static mut`", s.rust_static_mut_count
        )?;
    }
    if s.js_toplevel_count > 0 {
        writeln!(
            writer,
            "  {:35} {:>6}",
            "JS Top-level Mutables", s.js_toplevel_count
        )?;
    }

    for &kind in &[
        GlobalKind::ModuleCollection,
        GlobalKind::ClassVariable,
        GlobalKind::GlobalMutation,
        GlobalKind::RustStaticMut,
        GlobalKind::JsTopLevelMutable,
    ] {
        let by_kind: Vec<_> = result.matches.iter().filter(|m| m.kind == kind).collect();
        if by_kind.is_empty() {
            continue;
        }
        writeln!(
            writer,
            "───────────────────────────────────────────────────────────"
        )?;
        writeln!(writer, "  {} ({})", kind.description(), by_kind.len())?;
        let displayed = by_kind.iter().take(MAX_DISPLAYED_PER_KIND);
        for m in displayed {
            let rel = base
                .and_then(|b| m.file.strip_prefix(b).ok())
                .unwrap_or(&m.file);
            writeln!(
                writer,
                "    {}:{}:{}  {}  `{}`",
                rel.display(),
                m.line,
                m.column,
                m.name,
                m.snippet
            )?;
        }
        if by_kind.len() > MAX_DISPLAYED_PER_KIND {
            writeln!(
                writer,
                "    … and {} more",
                by_kind.len() - MAX_DISPLAYED_PER_KIND
            )?;
        }
    }

    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;
    Ok(())
}

/// Emits formatted JSON report to `writer`.
pub fn print_json_report<W: Write>(result: &GlobalsResult, writer: &mut W) -> Result<()> {
    serde_json::to_writer_pretty(writer, result)?;
    Ok(())
}
