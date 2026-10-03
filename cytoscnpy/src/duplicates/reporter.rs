//! Terminal and JSON reporters for duplicate code cluster analysis.

use anyhow::Result;
use std::io::Write;
use std::path::Path;

use super::types::DuplicatesResult;

const MAX_DISPLAYED_CLUSTERS: usize = 20;
const MAX_DISPLAYED_FILES: usize = 15;

/// Emits a human-readable terminal report to `writer`.
pub fn print_terminal_report<W: Write>(
    result: &DuplicatesResult,
    base: Option<&Path>,
    writer: &mut W,
) -> Result<()> {
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;
    writeln!(writer, "  Duplicate Code Clusters & Line Totals")?;
    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;

    if result.is_clean() {
        writeln!(writer, "  ✓ No duplicate clusters detected.")?;
        writeln!(
            writer,
            "═══════════════════════════════════════════════════════════"
        )?;
        return Ok(());
    }

    let s = &result.stats;
    writeln!(writer, "  Duplicate clusters  : {}", s.cluster_count)?;
    writeln!(
        writer,
        "  Duplicate lines     : {} (non-overlapping)",
        s.total_duplicate_lines
    )?;
    writeln!(writer, "  Total scanned lines : {}", s.total_scanned_lines)?;
    writeln!(writer, "  Duplication ratio   : {:.1}%", s.duplicate_pct)?;
    writeln!(writer, "  Affected files      : {}", s.affected_files)?;
    writeln!(writer, "  Files scanned       : {}", result.files_scanned)?;
    writeln!(
        writer,
        "───────────────────────────────────────────────────────────"
    )?;
    writeln!(writer, "  {:26} {:>6}", "Cluster Category", "Count")?;
    writeln!(
        writer,
        "  {:26} {:>6}",
        "──────────────────────────", "─────"
    )?;
    if s.exact_clusters > 0 {
        writeln!(
            writer,
            "  {:26} {:>6}",
            "Exact Copy (Type-1)", s.exact_clusters
        )?;
    }
    if s.renamed_clusters > 0 {
        writeln!(
            writer,
            "  {:26} {:>6}",
            "Renamed Copy (Type-2)", s.renamed_clusters
        )?;
    }
    if s.similar_clusters > 0 {
        writeln!(
            writer,
            "  {:26} {:>6}",
            "Similar Code (Type-3)", s.similar_clusters
        )?;
    }

    // Top clusters table
    writeln!(
        writer,
        "───────────────────────────────────────────────────────────"
    )?;
    writeln!(
        writer,
        "  Duplicate Clusters (Top {MAX_DISPLAYED_CLUSTERS})"
    )?;
    for cluster in result.clusters.iter().take(MAX_DISPLAYED_CLUSTERS) {
        writeln!(
            writer,
            "  Cluster #{} [{}] - {:.0}% match, ~{} lines, {} locations:",
            cluster.id,
            cluster.clone_type,
            cluster.similarity * 100.0,
            cluster.line_count,
            cluster.locations.len()
        )?;
        for loc in &cluster.locations {
            let rel = base
                .and_then(|b| loc.file.strip_prefix(b).ok())
                .unwrap_or(&loc.file);
            let name_str = loc
                .name
                .as_ref()
                .map(|n| format!(" ({n})"))
                .unwrap_or_default();
            writeln!(
                writer,
                "    - {}:{}-{} {}{}",
                rel.display(),
                loc.start_line,
                loc.end_line,
                loc.node_kind,
                name_str
            )?;
        }
    }
    if result.clusters.len() > MAX_DISPLAYED_CLUSTERS {
        writeln!(
            writer,
            "  … and {} more clusters",
            result.clusters.len() - MAX_DISPLAYED_CLUSTERS
        )?;
    }

    // Top files with duplicate code
    if !result.file_stats.is_empty() {
        writeln!(
            writer,
            "───────────────────────────────────────────────────────────"
        )?;
        writeln!(writer, "  Files with Most Duplicate Lines:")?;
        for f in result.file_stats.iter().take(MAX_DISPLAYED_FILES) {
            let rel = base
                .and_then(|b| f.file.strip_prefix(b).ok())
                .unwrap_or(&f.file);
            writeln!(
                writer,
                "    {:40} {:>5} dup lines ({:>4.1}%)",
                rel.display(),
                f.duplicate_lines,
                f.duplicate_pct
            )?;
        }
    }

    writeln!(
        writer,
        "═══════════════════════════════════════════════════════════"
    )?;
    Ok(())
}

/// Serializes duplicate analysis results to JSON.
pub fn print_json_report<W: Write>(result: &DuplicatesResult, writer: &mut W) -> Result<()> {
    let json = serde_json::to_string_pretty(result)?;
    writeln!(writer, "{json}")?;
    Ok(())
}
