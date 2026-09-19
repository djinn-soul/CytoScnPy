use anyhow::Result;
use std::io::Write;

use super::types::{ScoreResult, Verdict};

/// Format human-readable terminal report for the weighted Slop Index.
pub fn format_terminal_report(result: &ScoreResult) -> String {
    let mut out = String::new();

    out.push_str("\n+-------------------------------------------------------------------------+\n");
    out.push_str("|                        DESLOPIFY SLOP INDEX REPORT                      |\n");
    out.push_str("+-------------------------------------------------------------------------+\n");

    let verdict_badge = match result.verdict {
        Verdict::Clean => "[CLEAN - Pristine / Low LLM Friction]",
        Verdict::Acceptable => "[ACCEPTABLE - Normal Debt / Manageable]",
        Verdict::Messy => "[MESSY - Elevated Friction / High Debt]",
        Verdict::Sloppy => "[SLOPPY - Severe Friction / Degraded]",
        Verdict::Disaster => "[DISASTER - Critical Hazard / High Failure Risk]",
    };

    out.push_str(&format!(
        "|  Slop Index      : {:>3} / 100   {:<39} |\n",
        result.slop_index, verdict_badge
    ));
    out.push_str(&format!(
        "|  Raw Score       : {:>6.2} / 100                                                |\n",
        result.raw_score
    ));
    out.push_str(&format!(
        "|  Size Multiplier : {:>6.2}x (effective tokens vs context window)                |\n",
        result.size_multiplier
    ));
    out.push_str("+-------------------------------------------------------------------------+\n\n");

    out.push_str("Dimension Breakdown:\n");
    out.push_str(&format!(
        "  {:<26} {:>6} {:>8} {:>12}  {}\n",
        "DIMENSION", "WEIGHT", "RATING", "RAW POINTS", "EVIDENCE"
    ));
    out.push_str(&format!(
        "  {:-<26} {:-<6} {:-<8} {:-<12}  {:-<45}\n",
        "", "", "", "", ""
    ));

    for dim in &result.dimensions {
        let stars = format!("{}/5", dim.rating);
        out.push_str(&format!(
            "  {:<26} {:>6} {:>8} {:>12.2}  {}\n",
            dim.name, dim.weight, stars, dim.raw_contribution, dim.evidence
        ));
    }

    out.push('\n');

    if !result.recommendations.is_empty() {
        out.push_str("Top Prioritized Remediations:\n");
        for (i, rec) in result.recommendations.iter().take(5).enumerate() {
            out.push_str(&format!(
                "  {}. [-{} pts] {} ({}, Effort: {})\n",
                i + 1,
                rec.estimated_reduction,
                rec.title,
                rec.dimension,
                rec.effort
            ));
            if !rec.affected_files.is_empty() {
                let preview = rec
                    .affected_files
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                let more = if rec.affected_files.len() > 3 {
                    format!(" (+{} more)", rec.affected_files.len() - 3)
                } else {
                    String::new()
                };
                out.push_str(&format!("     Files: {preview}{more}\n"));
            }
        }
        out.push('\n');
    }

    if result.passed_gate {
        out.push_str("[GATE PASSED] Codebase satisfies slop index and threshold requirements.\n");
    } else {
        let reason = result
            .failure_reason
            .as_deref()
            .unwrap_or("Score threshold exceeded");
        out.push_str(&format!("[GATE FAILED] {reason}\n"));
    }

    out
}

/// Print human-readable terminal report to writer.
pub fn print_terminal_report<W: Write>(result: &ScoreResult, writer: &mut W) -> Result<()> {
    write!(writer, "{}", format_terminal_report(result))?;
    Ok(())
}

/// Print LLM-consumable markdown remediation report to writer.
pub fn print_llm_report<W: Write>(result: &ScoreResult, writer: &mut W) -> Result<()> {
    write!(
        writer,
        "{}",
        super::recommendations::format_llm_report(result)
    )?;
    Ok(())
}

/// Serialize and print JSON report to writer.
pub fn print_json_report<W: Write>(result: &ScoreResult, writer: &mut W) -> Result<()> {
    let json_str = serde_json::to_string_pretty(result)?;
    writeln!(writer, "{json_str}")?;
    Ok(())
}
