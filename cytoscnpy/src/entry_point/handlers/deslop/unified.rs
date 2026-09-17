//! Unified `DeSlopify` report orchestration.

use anyhow::Result;
use serde::Serialize;
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

mod gates;
mod output;

use gates::{collect_failures, GateFailure, GateSummary};

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
    searchability: crate::searchability::SearchabilityResult,
    naming: crate::naming::NamingDistributionResult,
    todos: crate::todos::TodosResult,
    globals: crate::globals::GlobalsResult,
    exceptions: crate::exceptions::ExceptionsResult,
    wildcards: crate::wildcards::WildcardsResult,
    side_effects: crate::side_effects::SideEffectsResult,
    singletons: crate::singletons::SingletonsResult,
    anti_patterns: crate::anti_patterns::AntiPatternsResult,
    duplicates: crate::duplicates::DuplicatesResult,
    unreferenced: crate::unreferenced::UnreferencedResult,
    gates: GateSummary,
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
    let searchability = crate::searchability::analyze_searchability(
        request.roots,
        request.excludes,
        request.cli.output.verbose,
    );
    let naming =
        crate::naming::analyze_naming(request.roots, request.excludes, request.cli.output.verbose);
    let todos =
        crate::todos::analyze_todos(request.roots, request.excludes, request.cli.output.verbose);
    let globals = crate::globals::analyze_globals(
        request.roots,
        request.excludes,
        request.cli.output.verbose,
    );
    let exceptions = crate::exceptions::analyze_exceptions(
        request.roots,
        request.excludes,
        request.cli.output.verbose,
    );
    let wildcards = crate::wildcards::analyze_wildcards(
        request.roots,
        request.excludes,
        request.cli.output.verbose,
    );
    let side_effects = crate::side_effects::analyze_side_effects(
        request.roots,
        request.excludes,
        request.cli.output.verbose,
    );
    let singletons = crate::singletons::analyze_singletons(
        request.roots,
        request.excludes,
        request.cli.output.verbose,
    );
    let anti_patterns = crate::anti_patterns::analyze_anti_patterns(
        request.roots,
        request.excludes,
        request.cli.output.verbose,
    );
    let include_tests = request.cli.include.include_tests
        || request.config.cytoscnpy.include_tests.unwrap_or(false);
    let duplicates = crate::duplicates::analyze_duplicates(
        request.roots,
        request.excludes,
        &crate::duplicates::DuplicatesOptions {
            include_tests,
            ..crate::duplicates::DuplicatesOptions::default()
        },
        request.cli.output.verbose,
    );
    let unreferenced = crate::unreferenced::analyze_unreferenced(
        request.roots,
        request.excludes,
        &crate::unreferenced::UnreferencedOptions {
            include_tests,
            ..crate::unreferenced::UnreferencedOptions::default()
        },
        request.cli.output.verbose,
    );

    let failures = collect_failures(
        &architecture,
        &context,
        &health,
        &searchability,
        &naming,
        &todos,
        &globals,
        &exceptions,
        &wildcards,
        &side_effects,
        &singletons,
        &anti_patterns,
        &duplicates,
        &unreferenced,
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
            &searchability,
            &naming,
            &todos,
            &globals,
            &exceptions,
            &wildcards,
            &side_effects,
            &singletons,
            &anti_patterns,
            &duplicates,
            &unreferenced,
            &failures,
        )?)
    };

    let report = ComprehensiveReport {
        schema_version: 1,
        architecture,
        context,
        health,
        searchability,
        naming,
        todos,
        globals,
        exceptions,
        wildcards,
        side_effects,
        singletons,
        anti_patterns,
        duplicates,
        unreferenced,
        gates: GateSummary {
            passed: failures.is_empty(),
            failures,
        },
    };
    output::write_report(
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
