use super::AnalysisContext;
use anyhow::Result;
use regex::Regex;
use std::io::Write;
use std::sync::OnceLock;

static MCCABE_RE: OnceLock<Option<Regex>> = OnceLock::new();

fn extract_mccabe_value(message: &str) -> Option<usize> {
    MCCABE_RE
        .get_or_init(|| Regex::new(r"McCabe\s*=\s*(\d+)").ok())
        .as_ref()
        .and_then(|re| re.captures(message))
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<usize>().ok())
}

pub(super) fn apply_complexity_gate<W: Write>(
    cli_var: &crate::cli::Cli,
    config: &crate::config::Config,
    result: &crate::analyzer::AnalysisResult,
    context: &AnalysisContext,
    writer: &mut W,
    exit_code: &mut i32,
) -> Result<()> {
    let Some(threshold) = cli_var.max_complexity.or(config.cytoscnpy.max_complexity) else {
        return Ok(());
    };

    match max_complexity_violation(result) {
        Some(max_found) if max_found > threshold => {
            if !context.is_structured {
                eprintln!("\n[GATE] Max complexity: {max_found} (threshold: {threshold}) - FAILED");
            }
            *exit_code = 1;
        }
        Some(max_found) if !context.is_structured => {
            writeln!(
                writer,
                "\n[GATE] Max complexity: {max_found} (threshold: {threshold}) - PASSED"
            )?;
        }
        None if !context.is_structured && !result.quality.is_empty() => {
            writeln!(
                writer,
                "\n[GATE] Max complexity: OK (threshold: {threshold}) - PASSED"
            )?;
        }
        _ => {}
    }

    Ok(())
}

fn max_complexity_violation(result: &crate::analyzer::AnalysisResult) -> Option<usize> {
    result
        .quality
        .iter()
        .filter(|f| f.rule_id == crate::rules::ids::RULE_ID_COMPLEXITY)
        .filter_map(|f| extract_mccabe_value(&f.message))
        .max()
}

pub(super) fn apply_mi_gate<W: Write>(
    cli_var: &crate::cli::Cli,
    config: &crate::config::Config,
    result: &crate::analyzer::AnalysisResult,
    context: &AnalysisContext,
    writer: &mut W,
    exit_code: &mut i32,
) -> Result<()> {
    let Some(threshold) = cli_var.min_mi.or(config.cytoscnpy.min_mi) else {
        return Ok(());
    };
    let mi = result
        .file_metrics
        .iter()
        .map(|metric| metric.mi)
        .fold(f64::INFINITY, f64::min);
    if result.file_metrics.is_empty() {
        return Ok(());
    }

    if mi < threshold {
        if !context.is_structured {
            eprintln!(
                "\n[GATE] Maintainability Index: {mi:.1} (threshold: {threshold:.1}) - FAILED"
            );
        }
        *exit_code = 1;
    } else if !context.is_structured {
        writeln!(
            writer,
            "\n[GATE] Maintainability Index: {mi:.1} (threshold: {threshold:.1}) - PASSED"
        )?;
    }

    Ok(())
}

pub(super) fn apply_quality_gate(
    cli_var: &crate::cli::Cli,
    config: &crate::config::Config,
    result: &crate::analyzer::AnalysisResult,
    context: &AnalysisContext,
    exit_code: &mut i32,
) {
    let thresholds = [
        (
            config.cytoscnpy.max_lines.is_some(),
            crate::rules::ids::RULE_ID_FUNCTION_LENGTH,
        ),
        (
            config.cytoscnpy.max_args.is_some(),
            crate::rules::ids::RULE_ID_ARGUMENT_COUNT,
        ),
        (
            config.cytoscnpy.max_nesting.is_some(),
            crate::rules::ids::RULE_ID_NESTING,
        ),
    ];
    let threshold_failed = thresholds.iter().any(|(enabled, rule)| {
        *enabled
            && result
                .quality
                .iter()
                .any(|finding| finding.rule_id == *rule)
    });
    if threshold_failed
        || ((cli_var.output.fail_on_any || cli_var.output.fail_on_quality)
            && !result.quality.is_empty())
    {
        if !context.is_structured {
            eprintln!(
                "\n[GATE] Quality issues: {} found - FAILED",
                result.quality.len()
            );
        }
        *exit_code = 1;
    }
}
