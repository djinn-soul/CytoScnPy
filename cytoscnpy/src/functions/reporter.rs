//! Terminal and JSON reporters for function metrics.

use anyhow::Result;
use std::io::Write;
use std::path::Path;

use super::types::FunctionsResult;

const MAX_DISPLAYED_FUNCTIONS: usize = 50;

/// Emits a human-readable terminal report to `writer`.
pub fn print_terminal_report<W: Write>(
    result: &FunctionsResult,
    base: Option<&Path>,
    writer: &mut W,
) -> Result<()> {
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;
    writeln!(writer, "  Function Metrics & Extraction Results")?;
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;

    if result.is_empty() {
        writeln!(writer, "  No functions detected.")?;
        writeln!(
            writer,
            "═══════════════════════════════════════════════════════════"
        )?;
        return Ok(());
    }

    let s = &result.stats;
    writeln!(writer, "  Total functions     : {}", s.total_functions)?;
    writeln!(writer, "  Files scanned       : {}", result.files_scanned)?;
    writeln!(
        writer,
        "  Avg / Max Complexity: {:.1} / {}",
        s.avg_complexity, s.max_complexity
    )?;
    writeln!(
        writer,
        "  Avg / Max Lines     : {:.1} / {}",
        s.avg_lines, s.max_lines
    )?;
    writeln!(
        writer,
        "  Avg / Max Nesting   : {:.1} / {}",
        s.avg_nesting, s.max_nesting
    )?;
    writeln!(
        writer,
        "───────────────────────────────────────────────────────────"
    )?;
    writeln!(
        writer,
        "  {:30} {:>6} {:>6} {:>7}  Location",
        "Function", "Lines", "CC", "Nesting"
    )?;
    writeln!(
        writer,
        "  {:30} {:>6} {:>6} {:>7}  ────────",
        "──────────────────────────────", "──────", "──────", "───────"
    )?;

    for (i, func) in result.functions.iter().enumerate() {
        if i >= MAX_DISPLAYED_FUNCTIONS {
            writeln!(
                writer,
                "  ... and {} more functions (use --json for complete list)",
                result.functions.len() - MAX_DISPLAYED_FUNCTIONS
            )?;
            break;
        }

        let display_path = if let Some(base_path) = base {
            func.file
                .strip_prefix(base_path)
                .unwrap_or(&func.file)
                .display()
                .to_string()
        } else {
            func.file.display().to_string()
        };

        let location = format!("{}:{}", display_path, func.start_line);
        let fn_name = if func.name.len() > 30 {
            format!("{}…", &func.name[..29])
        } else {
            func.name.clone()
        };

        writeln!(
            writer,
            "  {:30} {:>6} {:>6} {:>7}  {}",
            fn_name, func.line_count, func.cyclomatic_complexity, func.max_nesting, location
        )?;
    }

    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;
    Ok(())
}

/// Emits a JSON representation of `result` to `writer`.
pub fn print_json_report<W: Write>(result: &FunctionsResult, writer: &mut W) -> Result<()> {
    serde_json::to_writer_pretty(&mut *writer, result)?;
    writeln!(writer)?;
    Ok(())
}
