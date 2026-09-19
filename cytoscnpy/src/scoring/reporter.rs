use anyhow::Result;
use std::io::Write;

use super::types::{ScoreResult, Verdict};

/// Format human-readable terminal report for the weighted Slop Index.
pub fn format_terminal_report(result: &ScoreResult, verbose: bool) -> String {
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

    if let Some(summary) = &result.summary {
        out.push_str(&summary.format_terminal_summary());
    }

    if verbose {
        out.push_str("Dimension Breakdown (Complete):\n");
    } else {
        out.push_str("Dimension Breakdown:\n");
    }
    out.push_str(&format!(
        "  {:<26} {:>6} {:>8} {:>12}  {}\n",
        "DIMENSION", "WEIGHT", "RATING", "RAW POINTS", "EVIDENCE"
    ));
    out.push_str(&format!(
        "  {:-<26} {:-<6} {:-<8} {:-<12}  {:-<45}\n",
        "", "", "", "", ""
    ));

    let mut dims: Vec<&super::types::DimensionScore> = result.dimensions.iter().collect();
    if verbose {
        dims.sort_by(|a, b| {
            b.raw_contribution
                .partial_cmp(&a.raw_contribution)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.weight.cmp(&a.weight))
                .then_with(|| a.name.cmp(&b.name))
        });
    }

    for dim in &dims {
        let stars = format!("{}/5", dim.rating);
        out.push_str(&format!(
            "  {:<26} {:>6} {:>8} {:>12.2}  {}\n",
            dim.name, dim.weight, stars, dim.raw_contribution, dim.evidence
        ));
        if verbose {
            let status = match dim.rating {
                0 => "[PRISTINE] No friction detected",
                1 => "[LOW] Minor deviations",
                2 => "[MODERATE] Noticeable architectural friction",
                3 => "[SIGNIFICANT] Substantial friction / debt",
                4 => "[HIGH RISK] Severe architectural hazard",
                _ => "[CRITICAL RISK] Critical failure hazard",
            };
            out.push_str(&format!(
                "    -> Health Diagnostic: {status} | Rating: {}/5 | Raw Impact: {:.2} pts\n",
                dim.rating, dim.raw_contribution
            ));
        }
    }

    if verbose {
        let friction_dims: Vec<_> = result.dimensions.iter().filter(|d| d.rating > 0).collect();
        let friction_points: f64 = friction_dims.iter().map(|d| d.raw_contribution).sum();
        let pristine_count = result.dimensions.len().saturating_sub(friction_dims.len());
        out.push_str(&format!(
            "\n  Health Diagnostics Summary:\n    - Dimensions Analyzed : {} total\n    - Dimensions Clean   : {} pristine\n    - Dimensions At Risk : {} with friction ({:.2} raw debt)\n",
            result.dimensions.len(),
            pristine_count,
            friction_dims.len(),
            friction_points
        ));
    }

    out.push('\n');

    if !result.recommendations.is_empty() {
        out.push_str("Top Prioritized Remediations:\n");
        let rec_limit = if verbose { 10 } else { 5 };
        for (i, rec) in result.recommendations.iter().take(rec_limit).enumerate() {
            out.push_str(&format!(
                "  {}. [-{} pts] {} ({}, Effort: {})\n",
                i + 1,
                rec.estimated_reduction,
                rec.title,
                rec.dimension,
                rec.effort
            ));
            if !rec.affected_files.is_empty() {
                let file_limit = if verbose { 5 } else { 3 };
                let preview = rec
                    .affected_files
                    .iter()
                    .take(file_limit)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                let more = if rec.affected_files.len() > file_limit {
                    format!(" (+{} more)", rec.affected_files.len() - file_limit)
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
pub fn print_terminal_report<W: Write>(
    result: &ScoreResult,
    verbose: bool,
    writer: &mut W,
) -> Result<()> {
    write!(writer, "{}", format_terminal_report(result, verbose))?;
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
