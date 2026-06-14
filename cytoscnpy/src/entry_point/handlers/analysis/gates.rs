use anyhow::Result;
use regex::Regex;
use std::io::Write;
use std::sync::OnceLock;

use super::context::AnalysisContext;
use super::run::AnalysisRun;

static MCCABE_RE: OnceLock<Option<Regex>> = OnceLock::new();

fn resolve_gate(cli_flag: bool, config_flag: Option<bool>) -> bool {
    cli_flag || config_flag.unwrap_or(false)
}

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
    let mut exit_code = 0;

    apply_unused_code_gate(cli_var, config, result, context, writer, &mut exit_code)?;
    apply_complexity_gate(cli_var, config, result, context, writer, &mut exit_code)?;
    apply_mi_gate(cli_var, config, result, context, writer, &mut exit_code)?;
    apply_quality_gate(cli_var, result, context, &mut exit_code);
    apply_secrets_gate(cli_var, config, result, context, &mut exit_code);
    apply_danger_gate(cli_var, config, result, context, &mut exit_code);
    apply_missing_deps_gate(cli_var, config, result, context, &mut exit_code);
    apply_unused_deps_gate(cli_var, config, result, context, &mut exit_code);

    Ok(exit_code)
}

fn configured_fail_threshold(cli_var: &crate::cli::Cli, config: &crate::config::Config) -> f64 {
    cli_var
        .fail_threshold
        .or(config.cytoscnpy.fail_threshold)
        .or_else(|| {
            std::env::var("CYTOSCNPY_FAIL_THRESHOLD")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
        })
        .unwrap_or(100.0)
}

fn total_unused(result: &crate::analyzer::AnalysisResult) -> usize {
    result.unused_functions.len()
        + result.unused_methods.len()
        + result.unused_classes.len()
        + result.unused_imports.len()
        + result.unused_variables.len()
        + result.unused_parameters.len()
}

fn apply_unused_code_gate<W: Write>(
    cli_var: &crate::cli::Cli,
    config: &crate::config::Config,
    result: &crate::analyzer::AnalysisResult,
    context: &AnalysisContext,
    writer: &mut W,
    exit_code: &mut i32,
) -> Result<()> {
    if result.analysis_summary.total_definitions == 0 {
        return Ok(());
    }

    let fail_threshold = configured_fail_threshold(cli_var, config);
    #[allow(clippy::cast_precision_loss)] // Counts are far below 2^52.
    let percentage =
        (total_unused(result) as f64 / result.analysis_summary.total_definitions as f64) * 100.0;

    if percentage > fail_threshold {
        if !context.is_structured {
            eprintln!(
                "\n[GATE] Unused code: {percentage:.1}% (threshold: {fail_threshold:.1}%) - FAILED"
            );
        }
        *exit_code = 1;
    } else if fail_threshold < 100.0 && !context.is_structured {
        writeln!(
            writer,
            "\n[GATE] Unused code: {percentage:.1}% (threshold: {fail_threshold:.1}%) - PASSED"
        )?;
    }

    Ok(())
}

fn apply_complexity_gate<W: Write>(
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

fn apply_mi_gate<W: Write>(
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
    let mi = result.analysis_summary.average_mi;
    if mi <= 0.0 {
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

fn apply_quality_gate(
    cli_var: &crate::cli::Cli,
    result: &crate::analyzer::AnalysisResult,
    context: &AnalysisContext,
    exit_code: &mut i32,
) {
    if cli_var.output.fail_on_quality && !result.quality.is_empty() {
        if !context.is_structured {
            eprintln!(
                "\n[GATE] Quality issues: {} found - FAILED",
                result.quality.len()
            );
        }
        *exit_code = 1;
    }
}

fn apply_secrets_gate(
    cli_var: &crate::cli::Cli,
    config: &crate::config::Config,
    result: &crate::analyzer::AnalysisResult,
    context: &AnalysisContext,
    exit_code: &mut i32,
) {
    if resolve_gate(
        cli_var.output.fail_on_secrets,
        config.cytoscnpy.fail_on_secrets,
    ) && !result.secrets.is_empty()
    {
        if !context.is_structured {
            eprintln!(
                "\n[GATE] Secret findings: {} found - FAILED",
                result.secrets.len()
            );
        }
        *exit_code = 1;
    }
}

fn apply_danger_gate(
    cli_var: &crate::cli::Cli,
    config: &crate::config::Config,
    result: &crate::analyzer::AnalysisResult,
    context: &AnalysisContext,
    exit_code: &mut i32,
) {
    if resolve_gate(
        cli_var.output.fail_on_danger,
        config.cytoscnpy.fail_on_danger,
    ) && (!result.danger.is_empty() || !result.taint_findings.is_empty())
    {
        if !context.is_structured {
            eprintln!(
                "\n[GATE] Security findings: {} danger, {} taint - FAILED",
                result.danger.len(),
                result.taint_findings.len()
            );
        }
        *exit_code = 1;
    }
}

fn apply_missing_deps_gate(
    cli_var: &crate::cli::Cli,
    config: &crate::config::Config,
    result: &crate::analyzer::AnalysisResult,
    context: &AnalysisContext,
    exit_code: &mut i32,
) {
    if resolve_gate(
        cli_var.output.fail_on_missing_deps,
        config.cytoscnpy.deps.fail_on_missing,
    ) && !result.missing_dependencies.is_empty()
    {
        if !context.is_structured {
            eprintln!(
                "\n[GATE] Missing dependencies: {} found - FAILED",
                result.missing_dependencies.len()
            );
        }
        *exit_code = 1;
    }
}

fn apply_unused_deps_gate(
    cli_var: &crate::cli::Cli,
    config: &crate::config::Config,
    result: &crate::analyzer::AnalysisResult,
    context: &AnalysisContext,
    exit_code: &mut i32,
) {
    if resolve_gate(
        cli_var.output.fail_on_unused_deps,
        config.cytoscnpy.deps.fail_on_unused,
    ) && !result.unused_dependencies.is_empty()
    {
        if !context.is_structured {
            eprintln!(
                "\n[GATE] Unused dependencies: {} found - FAILED",
                result.unused_dependencies.len()
            );
        }
        *exit_code = 1;
    }
}
