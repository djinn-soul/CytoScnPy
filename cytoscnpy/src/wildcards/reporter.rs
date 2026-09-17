//! Terminal and JSON reporters for wildcard-import detection results.

use anyhow::Result;
use std::io::Write;
use std::path::Path;

use super::types::WildcardsResult;

/// Maximum number of matches displayed in terminal output.
const MAX_DISPLAYED: usize = 50;

/// Emits a human-readable terminal report to `writer`.
pub fn print_terminal_report<W: Write>(
    result: &WildcardsResult,
    base: Option<&Path>,
    writer: &mut W,
) -> Result<()> {
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;
    writeln!(writer, "  Wildcard Import Scan Results")?;
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;

    if result.is_clean() {
        writeln!(
            writer,
            "  ✓ No wildcard imports (from … import *) detected."
        )?;
        writeln!(
            writer,
            "═══════════════════════════════════════════════════════════"
        )?;
        return Ok(());
    }

    let s = &result.stats;
    writeln!(writer, "  Total wildcard imports : {}", s.total)?;
    writeln!(writer, "  Affected files          : {}", s.affected_files)?;
    writeln!(
        writer,
        "───────────────────────────────────────────────────────────"
    )?;

    let displayed = result.matches.iter().take(MAX_DISPLAYED);
    for m in displayed {
        let rel = base
            .and_then(|b| m.file.strip_prefix(b).ok())
            .unwrap_or(&m.file);
        writeln!(
            writer,
            "  {}:{}  from {} import *",
            rel.display(),
            m.line,
            m.module
        )?;
        if !m.snippet.is_empty() {
            writeln!(writer, "    {}", m.snippet)?;
        }
    }
    if result.matches.len() > MAX_DISPLAYED {
        writeln!(
            writer,
            "  … and {} more",
            result.matches.len() - MAX_DISPLAYED
        )?;
    }

    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;
    Ok(())
}

/// Serialises the result as pretty-printed JSON.
pub fn print_json_report<W: Write>(result: &WildcardsResult, writer: &mut W) -> Result<()> {
    let json = serde_json::to_string_pretty(result)?;
    writeln!(writer, "{json}")?;
    Ok(())
}
