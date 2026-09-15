//! Unified `DeSlopify` report orchestration.

use anyhow::Result;
use serde::Serialize;
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

mod output;

pub(super) struct ComprehensiveRequest<'a> {
    pub roots: &'a [PathBuf],
    pub analysis_root: &'a Path,
    pub excludes: &'a [String],
    pub args: &'a crate::cli::DeslopArgs,
    pub cli: &'a crate::cli::Cli,
    pub config: &'a crate::config::Config,
}

#[derive(Serialize)]
struct ComprehensiveReport {
    schema_version: u32,
    architecture: crate::architecture::ArchitectureGraphResult,
    context: crate::context::ContextAnalysisResult,
    health: Vec<crate::doctor::DoctorResult>,
    gates: GateSummary,
}

#[derive(Serialize)]
struct GateSummary {
    passed: bool,
    failures: Vec<GateFailure>,
}

#[derive(Serialize)]
struct GateFailure {
    check: &'static str,
    actual: String,
    limit: String,
}

pub(super) fn run_comprehensive_deslop<W: Write>(
    request: &ComprehensiveRequest<'_>,
    writer: &mut W,
) -> Result<i32> {
    let architecture = crate::architecture::analyze_architecture(
        request.roots,
        request.excludes,
        request.cli.output.verbose,
    );
    let context = crate::context::analyze_context(
        request.roots,
        request.excludes,
        &crate::context::ContextConfig {
            git_months: request.args.git_months,
            context_budget: request.args.context_budget,
            no_git: request.args.no_git,
            verbose: request.cli.output.verbose,
            ..crate::context::ContextConfig::default()
        },
    );
    let health = health_results(request);

    let failures = collect_failures(
        &architecture,
        &context,
        &health,
        request.config,
        request.cli.output.fail_on_any,
    );
    let exit_code = i32::from(!failures.is_empty());
    let human_output = if request.cli.output.json {
        None
    } else {
        Some(output::render_human_report(
            &architecture,
            &context,
            &health,
            &failures,
        )?)
    };

    let report = ComprehensiveReport {
        schema_version: 1,
        architecture,
        context,
        health,
        gates: GateSummary {
            passed: failures.is_empty(),
            failures,
        },
    };
    write_report(
        &report,
        human_output.as_deref(),
        request.args,
        request.analysis_root,
        request.cli.output.json,
        writer,
    )?;
    Ok(exit_code)
}

fn health_results(request: &ComprehensiveRequest<'_>) -> Vec<crate::doctor::DoctorResult> {
    let mut seen = HashSet::new();
    request
        .roots
        .iter()
        .map(|path| {
            if path.is_file() {
                path.parent().unwrap_or(request.analysis_root)
            } else {
                path.as_path()
            }
        })
        .filter(|path| seen.insert((*path).to_path_buf()))
        .map(|path| {
            crate::doctor::run_doctor(
                path,
                &crate::doctor::DoctorConfig {
                    excludes: request.excludes.to_vec(),
                    verbose: request.cli.output.verbose,
                    fail_on_missing: false,
                },
            )
        })
        .collect()
}

fn collect_failures(
    architecture: &crate::architecture::ArchitectureGraphResult,
    context: &crate::context::ContextAnalysisResult,
    health: &[crate::doctor::DoctorResult],
    config: &crate::config::Config,
    fail_on_any: bool,
) -> Vec<GateFailure> {
    let mut failures = Vec::new();
    if !fail_on_any {
        return failures;
    }

    let deslop = &config.cytoscnpy.deslop;
    push_over(
        &mut failures,
        "circular_dependencies",
        architecture.stats.circular_dependency_count,
        deslop.max_cycles,
    );
    push_over(
        &mut failures,
        "god_modules",
        architecture.stats.god_modules.len(),
        deslop.max_god_modules,
    );
    let severe_hotspots = context
        .hotspots
        .iter()
        .filter(|item| {
            matches!(
                item.risk_level,
                crate::context::HotspotRiskLevel::High | crate::context::HotspotRiskLevel::Critical
            )
        })
        .count();
    push_over(
        &mut failures,
        "severe_hotspots",
        severe_hotspots,
        deslop.max_hotspots,
    );
    for result in health {
        if result.reliability.score < deslop.min_health_score {
            failures.push(failure(
                "health_score",
                result.reliability.score.to_string(),
                format!(">= {}", deslop.min_health_score),
            ));
        }
    }
    if let Some(limit) = deslop.max_navigation_pct {
        if context.token_budget.navigation_pct > limit {
            failures.push(failure(
                "navigation_pct",
                format!("{:.1}", context.token_budget.navigation_pct),
                format!("<= {limit:.1}"),
            ));
        }
    }
    failures
}

fn push_over(failures: &mut Vec<GateFailure>, check: &'static str, actual: usize, limit: usize) {
    if actual > limit {
        failures.push(failure(check, actual.to_string(), format!("<= {limit}")));
    }
}

fn failure(check: &'static str, actual: String, limit: String) -> GateFailure {
    GateFailure {
        check,
        actual,
        limit,
    }
}

fn write_report<W: Write>(
    report: &ComprehensiveReport,
    human_output: Option<&str>,
    args: &crate::cli::DeslopArgs,
    analysis_root: &Path,
    json: bool,
    writer: &mut W,
) -> Result<()> {
    let payload = match human_output {
        Some(output) => output.to_owned(),
        None => serde_json::to_string_pretty(report)?,
    };
    if let Some(output) = &args.output {
        let path = crate::utils::validate_output_path(Path::new(output), Some(analysis_root))?;
        std::fs::write(&path, payload)?;
        if !json {
            writeln!(writer, "Report written to: {}", path.display())?;
        }
    } else {
        writeln!(writer, "{payload}")?;
    }
    Ok(())
}
