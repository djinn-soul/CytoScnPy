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
    slop_index: u32,
    verdict: crate::scoring::Verdict,
    raw_score: f64,
    size_multiplier: f64,
    dimensions: Vec<crate::scoring::DimensionScore>,
    recommendations: Vec<crate::scoring::Recommendation>,
    scoring: crate::scoring::ScoreResult,
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

    let doc_root = health
        .first()
        .map_or(request.analysis_root, |d| d.root_path.as_path());
    let aggregated_health = crate::doctor::aggregate_doctor_results(&health, doc_root);

    let scoring_ctx = crate::scoring::ScoringContext {
        architecture: &architecture,
        context: &context,
        doctor: aggregated_health.as_ref(),
        searchability: &searchability,
        naming: &naming,
        todos: &todos,
        globals: &globals,
        exceptions: &exceptions,
        wildcards: &wildcards,
        side_effects: &side_effects,
        singletons: &singletons,
        anti_patterns: &anti_patterns,
        duplicates: &duplicates,
        unreferenced: &unreferenced,
    };
    let max_score = if request.cli.output.fail_on_any {
        request.config.cytoscnpy.deslop.max_slop_index
    } else {
        None
    };
    let scoring = crate::scoring::score_repository(
        &scoring_ctx,
        &crate::scoring::ScoringOptions {
            max_score,
            fail_on_any: false,
            ci: false,
            context_budget: request.args.context_budget,
            no_git: request.args.no_git,
            git_months: request.args.git_months,
        },
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
        &scoring,
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
            &scoring,
            &failures,
        )?)
    };

    let report = ComprehensiveReport {
        schema_version: 1,
        slop_index: scoring.slop_index,
        verdict: scoring.verdict,
        raw_score: scoring.raw_score,
        size_multiplier: scoring.size_multiplier,
        dimensions: scoring.dimensions.clone(),
        recommendations: scoring.recommendations.clone(),
        scoring,
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
        .map(|path| crate::doctor::resolve_doctor_target(path, request.analysis_root))
        .filter(|path| seen.insert(path.clone()))
        .map(|path| {
            crate::doctor::run_doctor(
                &path,
                &crate::doctor::DoctorConfig {
                    excludes: request.excludes.to_vec(),
                    verbose: request.cli.output.verbose,
                    fail_on_missing: false,
                },
            )
        })
        .collect()
}
