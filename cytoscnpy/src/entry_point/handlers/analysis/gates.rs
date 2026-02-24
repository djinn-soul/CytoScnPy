use anyhow::Result;
use regex::Regex;
use std::sync::OnceLock;

use super::context::AnalysisContext;
use super::run::AnalysisRun;

static MCCABE_RE: OnceLock<Option<Regex>> = OnceLock::new();

fn extract_mccabe_value(message: &str) -> Option<usize> {
    MCCABE_RE
        .get_or_init(|| Regex::new(r"McCabe\s*=\s*(\d+)").ok())
        .as_ref()
        .and_then(|re| re.captures(message))
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<usize>().ok())
}

pub(crate) fn apply_gates<W: std::io::Write>(
    cli_var: &crate::cli::Cli,
    config: &crate::config::Config,
    _analysis_root: &std::path::Path,
    context: &AnalysisContext,
    run: &AnalysisRun,
    writer: &mut W,
) -> Result<i32> {
    let result = &run.result;

    // Check for fail threshold (CLI > config > env var > default)
    let fail_threshold = cli_var
        .fail_threshold
        .or(config.cytoscnpy.fail_threshold)
        .or_else(|| {
            std::env::var("CYTOSCNPY_FAIL_THRESHOLD")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
        })
        .unwrap_or(100.0); // Default to 100% (never fail unless explicitly set)

    let mut exit_code = 0;

    // Calculate unused percentage and show gate status
    if result.analysis_summary.total_definitions > 0 {
        let total_unused = result.unused_functions.len()
            + result.unused_methods.len()
            + result.unused_classes.len()
            + result.unused_imports.len()
            + result.unused_variables.len()
            + result.unused_parameters.len();

        #[allow(clippy::cast_precision_loss)] // Counts are far below 2^52
        let percentage =
            (total_unused as f64 / result.analysis_summary.total_definitions as f64) * 100.0;

        // Only show gate banner if threshold is configured (not default 100%)
        let show_gate = fail_threshold < 100.0;

        if percentage > fail_threshold {
            if !context.is_structured {
                eprintln!(
                    "\n[GATE] Unused code: {percentage:.1}% (threshold: {fail_threshold:.1}%) - FAILED"
                );
            }

            exit_code = 1;
        } else if show_gate && !context.is_structured {
            writeln!(
                writer,
                "\n[GATE] Unused code: {percentage:.1}% (threshold: {fail_threshold:.1}%) - PASSED"
            )?;
        }
    }

    // Complexity gate check
    let max_complexity = cli_var.max_complexity.or(config.cytoscnpy.max_complexity);
    if let Some(threshold) = max_complexity {
        // Find the highest complexity violation
        let complexity_violations: Vec<usize> = result
            .quality
            .iter()
            .filter(|f| f.rule_id == crate::rules::ids::RULE_ID_COMPLEXITY)
            .filter_map(|f| extract_mccabe_value(&f.message))
            .collect();

        if let Some(&max_found) = complexity_violations.iter().max() {
            if max_found > threshold {
                if !cli_var.output.json {
                    eprintln!(
                        "\n[GATE] Max complexity: {max_found} (threshold: {threshold}) - FAILED"
                    );
                }
                exit_code = 1;
            } else if !cli_var.output.json {
                writeln!(
                    writer,
                    "\n[GATE] Max complexity: {max_found} (threshold: {threshold}) - PASSED"
                )?;
            }
        } else if !cli_var.output.json && !result.quality.is_empty() {
            // No complexity violations found, all functions are below threshold
            writeln!(
                writer,
                "\n[GATE] Max complexity: OK (threshold: {threshold}) - PASSED"
            )?;
        }
    }

    // Maintainability Index gate check
    let min_mi = cli_var.min_mi.or(config.cytoscnpy.min_mi);
    if let Some(threshold) = min_mi {
        let mi = result.analysis_summary.average_mi;
        if mi > 0.0 {
            if mi < threshold {
                if !cli_var.output.json {
                    eprintln!(
                        "\n[GATE] Maintainability Index: {mi:.1} (threshold: {threshold:.1}) - FAILED"
                    );
                }
                exit_code = 1;
            } else if !cli_var.output.json {
                writeln!(
                    writer,
                    "\n[GATE] Maintainability Index: {mi:.1} (threshold: {threshold:.1}) - PASSED"
                )?;
            }
        }
    }

    // Quality gate check (--fail-on-quality)
    if cli_var.output.fail_on_quality && !result.quality.is_empty() {
        if !cli_var.output.json {
            eprintln!(
                "\n[GATE] Quality issues: {} found - FAILED",
                result.quality.len()
            );
        }
        exit_code = 1;
    }

    Ok(exit_code)
}
