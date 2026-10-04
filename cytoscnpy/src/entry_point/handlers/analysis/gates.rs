use anyhow::Result;
use std::io::Write;

use super::context::AnalysisContext;
use super::run::AnalysisRun;

#[path = "quality_gates.rs"]
mod quality_gates;
use quality_gates::{apply_complexity_gate, apply_mi_gate, apply_quality_gate};

fn resolve_gate(cli_flag: bool, config_flag: Option<bool>) -> bool {
    cli_flag || config_flag.unwrap_or(false)
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
    if !result.parse_errors.is_empty() {
        if !context.is_structured {
            eprintln!(
                "\n[GATE] Scan incomplete: {} parsing/read errors - FAILED",
                result.parse_errors.len()
            );
        }
        exit_code = 1;
    }

    apply_unused_code_gate(cli_var, config, result, context, writer, &mut exit_code)?;
    apply_complexity_gate(cli_var, config, result, context, writer, &mut exit_code)?;
    apply_mi_gate(cli_var, config, result, context, writer, &mut exit_code)?;
    apply_quality_gate(cli_var, config, result, context, &mut exit_code);
    apply_secrets_gate(cli_var, config, result, context, &mut exit_code);
    apply_danger_gate(cli_var, config, result, context, &mut exit_code);
    apply_missing_deps_gate(cli_var, config, result, context, &mut exit_code);
    apply_unused_deps_gate(cli_var, config, result, context, &mut exit_code);

    Ok(exit_code)
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

    let fail_threshold = config
        .cytoscnpy
        .fail_threshold
        .unwrap_or(if cli_var.output.fail_on_any {
            0.0
        } else {
            100.0
        });
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

fn apply_secrets_gate(
    cli_var: &crate::cli::Cli,
    config: &crate::config::Config,
    result: &crate::analyzer::AnalysisResult,
    context: &AnalysisContext,
    exit_code: &mut i32,
) {
    if resolve_gate(
        cli_var.output.fail_on_any || cli_var.output.fail_on_secrets,
        config.cytoscnpy.fail_on_secrets,
    ) && !result.secrets.is_empty()
    {
        if !context.is_structured {
            eprintln!("\n[GATE] Secret findings detected - FAILED");
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
        cli_var.output.fail_on_any || cli_var.output.fail_on_danger,
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
        cli_var.output.fail_on_any || cli_var.output.fail_on_missing_deps,
        config.cytoscnpy.deps.fail_on_missing,
    ) && !(result.missing_dependencies.is_empty()
        && result.transitive_dependencies.is_empty()
        && result.dev_dependencies_in_production.is_empty())
    {
        if !context.is_structured {
            eprintln!(
                "\n[GATE] Missing dependencies: {} found - FAILED",
                result.missing_dependencies.len()
                    + result.transitive_dependencies.len()
                    + result.dev_dependencies_in_production.len()
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
        cli_var.output.fail_on_any || cli_var.output.fail_on_unused_deps,
        config.cytoscnpy.deps.fail_on_unused,
    ) && !(result.unused_dependencies.is_empty() && result.stdlib_dependencies.is_empty())
    {
        if !context.is_structured {
            eprintln!(
                "\n[GATE] Unused dependencies: {} found - FAILED",
                result.unused_dependencies.len() + result.stdlib_dependencies.len()
            );
        }
        *exit_code = 1;
    }
}
