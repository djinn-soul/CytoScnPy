//! Terminal and JSON reporters for exception-handler anti-pattern results.

use anyhow::Result;
use std::io::Write;
use std::path::Path;

use super::types::{ExceptionKind, ExceptionsResult};

/// Maximum number of matches displayed per kind in terminal output.
const MAX_DISPLAYED_PER_KIND: usize = 30;

/// Emits a human-readable terminal report to `writer`.
pub fn print_terminal_report<W: Write>(
    result: &ExceptionsResult,
    base: Option<&Path>,
    writer: &mut W,
) -> Result<()> {
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;
    writeln!(writer, "  Exception Handler Anti-Pattern Results")?;
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;

    if result.is_clean() {
        writeln!(
            writer,
            "  ✓ No bare-except or empty exception handlers detected."
        )?;
        writeln!(
            writer,
            "═══════════════════════════════════════════════════════════"
        )?;
        return Ok(());
    }

    let s = &result.stats;
    writeln!(writer, "  Total anti-patterns : {}", s.total)?;
    writeln!(writer, "  Affected files       : {}", s.affected_files)?;
    writeln!(
        writer,
        "───────────────────────────────────────────────────────────"
    )?;
    writeln!(writer, "  {:24} {:>6}", "Kind", "Count")?;
    writeln!(writer, "  {:24} {:>6}", "──────────────────────", "─────")?;
    if s.bare_except_count > 0 {
        writeln!(writer, "  {:24} {:>6}", "bare_except", s.bare_except_count)?;
    }
    if s.empty_handler_count > 0 {
        writeln!(
            writer,
            "  {:24} {:>6}",
            "empty_handler", s.empty_handler_count
        )?;
    }

    for &kind in &[ExceptionKind::BareExcept, ExceptionKind::EmptyHandler] {
        let by_kind: Vec<_> = result.matches.iter().filter(|m| m.kind == kind).collect();
        if by_kind.is_empty() {
            continue;
        }
        writeln!(
            writer,
            "───────────────────────────────────────────────────────────"
        )?;
        writeln!(writer, "  {}  ({})", kind.description(), by_kind.len())?;
        let displayed = by_kind.iter().take(MAX_DISPLAYED_PER_KIND);
        for m in displayed {
            let rel = base
                .and_then(|b| m.file.strip_prefix(b).ok())
                .unwrap_or(&m.file);
            writeln!(writer, "    {}:{}  {}", rel.display(), m.line, m.snippet)?;
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

/// Serialises the result as pretty-printed JSON.
pub fn print_json_report<W: Write>(result: &ExceptionsResult, writer: &mut W) -> Result<()> {
    let json = serde_json::to_string_pretty(result)?;
    writeln!(writer, "{json}")?;
    Ok(())
}
