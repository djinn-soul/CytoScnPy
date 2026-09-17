//! Terminal and JSON reporters for module-level side-effects analysis results.

use anyhow::Result;
use std::io::Write;
use std::path::Path;

use super::types::{SideEffectKind, SideEffectsResult};

const MAX_DISPLAYED_PER_KIND: usize = 25;

/// Emits a human-readable terminal report to `writer`.
pub fn print_terminal_report<W: Write>(
    result: &SideEffectsResult,
    base: Option<&Path>,
    writer: &mut W,
) -> Result<()> {
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;
    writeln!(writer, "  Module-Level Side Effects Scan Results")?;
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;

    if result.is_clean() {
        writeln!(
            writer,
            "  ✓ No module-level import-time side effects detected."
        )?;
        writeln!(
            writer,
            "═══════════════════════════════════════════════════════════"
        )?;
        return Ok(());
    }

    let s = &result.stats;
    writeln!(writer, "  Total side effects  : {}", s.total)?;
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

    if s.python_calls > 0 {
        writeln!(
            writer,
            "  {:26} {:>6}",
            "python_toplevel_call", s.python_calls
        )?;
    }
    if s.python_loops > 0 {
        writeln!(
            writer,
            "  {:26} {:>6}",
            "python_toplevel_loop", s.python_loops
        )?;
    }
    if s.python_withs > 0 {
        writeln!(
            writer,
            "  {:26} {:>6}",
            "python_toplevel_with", s.python_withs
        )?;
    }
    if s.js_calls > 0 {
        writeln!(writer, "  {:26} {:>6}", "js_toplevel_call", s.js_calls)?;
    }
    if s.js_event_listeners > 0 {
        writeln!(
            writer,
            "  {:26} {:>6}",
            "js_event_listener", s.js_event_listeners
        )?;
    }

    let all_kinds = [
        SideEffectKind::PythonTopLevelCall,
        SideEffectKind::PythonTopLevelLoop,
        SideEffectKind::PythonTopLevelWith,
        SideEffectKind::JsTopLevelCall,
        SideEffectKind::JsEventListener,
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
                "    {}:{}:{}  {}",
                rel.display(),
                m.line,
                m.column,
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

/// Serialises the result as pretty-printed JSON.
pub fn print_json_report<W: Write>(result: &SideEffectsResult, writer: &mut W) -> Result<()> {
    let json = serde_json::to_string_pretty(result)?;
    writeln!(writer, "{json}")?;
    Ok(())
}
