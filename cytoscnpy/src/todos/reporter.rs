//! Terminal and JSON reporters for the `todos` detection results.

use anyhow::Result;
use std::io::Write;
use std::path::Path;

use super::types::{TodoKind, TodosResult};

/// Maximum number of matches displayed per kind in terminal output.
const MAX_DISPLAYED_PER_KIND: usize = 30;

/// Emits a human-readable terminal report to `writer`.
pub fn print_terminal_report<W: Write>(
    result: &TodosResult,
    base: Option<&Path>,
    writer: &mut W,
) -> Result<()> {
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;
    writeln!(writer, "  TODO / Annotation Scan Results")?;
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;

    if result.is_clean() {
        writeln!(
            writer,
            "  ✓ No TODO/FIXME/HACK/XXX or debug prints detected."
        )?;
        writeln!(
            writer,
            "═══════════════════════════════════════════════════════════"
        )?;
        return Ok(());
    }

    let s = &result.stats;
    writeln!(writer, "  Total annotations : {}", s.total)?;
    writeln!(writer, "  Affected files     : {}", s.affected_files)?;
    writeln!(
        writer,
        "───────────────────────────────────────────────────────────"
    )?;
    writeln!(writer, "  {:20} {:>6}", "Kind", "Count")?;
    writeln!(writer, "  {:20} {:>6}", "──────────────────", "─────")?;
    if s.todo_count > 0 {
        writeln!(writer, "  {:20} {:>6}", "TODO", s.todo_count)?;
    }
    if s.fixme_count > 0 {
        writeln!(writer, "  {:20} {:>6}", "FIXME", s.fixme_count)?;
    }
    if s.hack_count > 0 {
        writeln!(writer, "  {:20} {:>6}", "HACK", s.hack_count)?;
    }
    if s.xxx_count > 0 {
        writeln!(writer, "  {:20} {:>6}", "XXX", s.xxx_count)?;
    }
    if s.debug_print_count > 0 {
        writeln!(writer, "  {:20} {:>6}", "debug_print", s.debug_print_count)?;
    }
    if s.commented_code_count > 0 {
        writeln!(
            writer,
            "  {:20} {:>6}",
            "commented_code", s.commented_code_count
        )?;
    }

    // Print matches grouped by kind, capped for readability
    for &kind in &[
        TodoKind::Todo,
        TodoKind::Fixme,
        TodoKind::Hack,
        TodoKind::Xxx,
        TodoKind::DebugPrint,
        TodoKind::CommentedCode,
    ] {
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
pub fn print_json_report<W: Write>(result: &TodosResult, writer: &mut W) -> Result<()> {
    let json = serde_json::to_string_pretty(result)?;
    writeln!(writer, "{json}")?;
    Ok(())
}
