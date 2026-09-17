//! Terminal and JSON reporters for anti-pattern detection results.

use anyhow::Result;
use std::io::Write;
use std::path::Path;

use super::types::{AntiPatternKind, AntiPatternsResult};

const MAX_DISPLAYED_PER_KIND: usize = 25;

/// Emits a human-readable terminal report to `writer`.
pub fn print_terminal_report<W: Write>(
    result: &AntiPatternsResult,
    base: Option<&Path>,
    writer: &mut W,
) -> Result<()> {
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;
    writeln!(writer, "  Anti-Pattern Scan Results")?;
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;

    if result.is_clean() {
        writeln!(writer, "  ✓ No anti-patterns detected.")?;
        writeln!(
            writer,
            "═══════════════════════════════════════════════════════════"
        )?;
        return Ok(());
    }

    let s = &result.stats;
    writeln!(writer, "  Total anti-patterns : {}", s.total)?;
    writeln!(writer, "  Affected files      : {}", s.affected_files)?;
    writeln!(writer, "  Files scanned       : {}", result.files_scanned)?;
    writeln!(
        writer,
        "───────────────────────────────────────────────────────────"
    )?;
    writeln!(writer, "  {:26} {:>6}", "Kind", "Count")?;
    writeln!(
        writer,
        "  {:26} {:>6}",
        "──────────────────────────", "─────"
    )?;

    if s.magic_numbers > 0 {
        writeln!(writer, "  {:26} {:>6}", "magic_number", s.magic_numbers)?;
    }
    if s.nested_callbacks > 0 {
        writeln!(
            writer,
            "  {:26} {:>6}",
            "deeply_nested_callback", s.nested_callbacks
        )?;
    }

    let all_kinds = [
        AntiPatternKind::MagicNumber,
        AntiPatternKind::DeeplyNestedCallback,
    ];

    for kind in all_kinds {
        let by_kind: Vec<_> = result.matches.iter().filter(|m| m.kind == kind).collect();
        if by_kind.is_empty() {
            continue;
        }
        writeln!(
            writer,
            "───────────────────────────────────────────────────────────"
        )?;
        writeln!(writer, "  {}  ({})", kind.description(), by_kind.len())?;
        for m in by_kind.iter().take(MAX_DISPLAYED_PER_KIND) {
            let rel = base
                .and_then(|b| m.file.strip_prefix(b).ok())
                .unwrap_or(&m.file);
            writeln!(
                writer,
                "    {}:{}:{}  {} ({})",
                rel.display(),
                m.line,
                m.column,
                m.pattern_name,
                m.kind
            )?;
            if !m.snippet.is_empty() {
                writeln!(writer, "      {}", m.snippet)?;
            }
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
pub fn print_json_report<W: Write>(result: &AntiPatternsResult, writer: &mut W) -> Result<()> {
    let json = serde_json::to_string_pretty(result)?;
    writeln!(writer, "{json}")?;
    Ok(())
}
