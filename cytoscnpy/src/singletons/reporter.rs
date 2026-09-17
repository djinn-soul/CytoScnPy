//! Terminal and JSON reporters for singleton pattern detection results.

use anyhow::Result;
use std::io::Write;
use std::path::Path;

use super::types::{SingletonKind, SingletonsResult};

const MAX_DISPLAYED_PER_KIND: usize = 25;

/// Emits a human-readable terminal report to `writer`.
pub fn print_terminal_report<W: Write>(
    result: &SingletonsResult,
    base: Option<&Path>,
    writer: &mut W,
) -> Result<()> {
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;
    writeln!(writer, "  Singleton Pattern Scan Results")?;
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;

    if result.is_clean() {
        writeln!(writer, "  ✓ No singleton patterns detected.")?;
        writeln!(
            writer,
            "═══════════════════════════════════════════════════════════"
        )?;
        return Ok(());
    }

    let s = &result.stats;
    writeln!(writer, "  Total singletons    : {}", s.total)?;
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

    if s.new_methods > 0 {
        writeln!(writer, "  {:26} {:>6}", "new_method", s.new_methods)?;
    }
    if s.get_instance_methods > 0 {
        writeln!(
            writer,
            "  {:26} {:>6}",
            "get_instance", s.get_instance_methods
        )?;
    }
    if s.decorators > 0 {
        writeln!(writer, "  {:26} {:>6}", "decorator", s.decorators)?;
    }
    if s.metaclasses > 0 {
        writeln!(writer, "  {:26} {:>6}", "metaclass", s.metaclasses)?;
    }

    let all_kinds = [
        SingletonKind::NewMethod,
        SingletonKind::GetInstanceMethod,
        SingletonKind::Decorator,
        SingletonKind::Metaclass,
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
                "    {}:{}:{}  class {} ({})",
                rel.display(),
                m.line,
                m.column,
                m.class_name,
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
pub fn print_json_report<W: Write>(result: &SingletonsResult, writer: &mut W) -> Result<()> {
    let json = serde_json::to_string_pretty(result)?;
    writeln!(writer, "{json}")?;
    Ok(())
}
