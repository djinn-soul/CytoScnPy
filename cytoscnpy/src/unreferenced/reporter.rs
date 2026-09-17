//! Terminal and JSON reporting for unreferenced large function analysis.

use anyhow::Result;
use std::io::Write;
use std::path::Path;

use super::types::UnreferencedResult;

const MAX_DISPLAYED_FUNCTIONS: usize = 25;
const MAX_DISPLAYED_FILES: usize = 15;

/// Emits human-readable terminal report to `writer`.
pub fn print_terminal_report<W: Write>(
    result: &UnreferencedResult,
    base: Option<&Path>,
    writer: &mut W,
) -> Result<()> {
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;
    writeln!(writer, "  Unreferenced Large Functions in Isolated Files")?;
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;

    if result.is_clean() {
        writeln!(
            writer,
            "  ✓ No unreferenced large functions detected in isolated files."
        )?;
        writeln!(
            writer,
            "═══════════════════════════════════════════════════════════"
        )?;
        return Ok(());
    }

    let s = &result.stats;
    writeln!(
        writer,
        "  Unreferenced functions : {}",
        s.total_unreferenced_functions
    )?;
    writeln!(
        writer,
        "  Unreferenced lines     : {}",
        s.total_unreferenced_lines
    )?;
    writeln!(
        writer,
        "  Isolated files         : {}",
        s.isolated_files_count
    )?;
    writeln!(
        writer,
        "  Files scanned          : {}",
        s.total_scanned_files
    )?;
    writeln!(
        writer,
        "  Total scanned lines    : {}",
        s.total_scanned_lines
    )?;
    writeln!(
        writer,
        "───────────────────────────────────────────────────────────"
    )?;
    writeln!(
        writer,
        "  Top Unreferenced Functions (Top {MAX_DISPLAYED_FUNCTIONS}):"
    )?;

    for func in result.items.iter().take(MAX_DISPLAYED_FUNCTIONS) {
        let rel = base
            .and_then(|b| func.file.strip_prefix(b).ok())
            .unwrap_or(&func.file);

        let class_str = func
            .class_name
            .as_ref()
            .map(|c| format!(" (class {c})"))
            .unwrap_or_default();

        writeln!(
            writer,
            "    - {}:{}-{} {} {}{} ({} lines)",
            rel.display(),
            func.start_line,
            func.end_line,
            func.node_kind,
            func.name,
            class_str,
            func.line_count,
        )?;
    }

    if result.items.len() > MAX_DISPLAYED_FUNCTIONS {
        writeln!(
            writer,
            "    … and {} more functions",
            result.items.len() - MAX_DISPLAYED_FUNCTIONS
        )?;
    }

    if !result.isolated_files.is_empty() {
        writeln!(
            writer,
            "───────────────────────────────────────────────────────────"
        )?;
        writeln!(
            writer,
            "  Isolated Files with Unreferenced Functions (Top {MAX_DISPLAYED_FILES}):"
        )?;
        for file in result.isolated_files.iter().take(MAX_DISPLAYED_FILES) {
            let rel = base
                .and_then(|b| file.file.strip_prefix(b).ok())
                .unwrap_or(&file.file);
            writeln!(
                writer,
                "    {:40} {:>3} funcs, {:>4} lines (total: {})",
                rel.display(),
                file.unreferenced_functions,
                file.unreferenced_lines,
                file.total_lines,
            )?;
        }
    }

    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;
    Ok(())
}

/// Serializes unreferenced function analysis results to JSON.
pub fn print_json_report<W: Write>(result: &UnreferencedResult, writer: &mut W) -> Result<()> {
    let json = serde_json::to_string_pretty(result)?;
    writeln!(writer, "{json}")?;
    Ok(())
}
