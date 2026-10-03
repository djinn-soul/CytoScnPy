//! Unified `DeSlopify` report orchestration.

use anyhow::Result;
use serde::Serialize;
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

mod gates;
mod inventory;
mod output;

use gates::{collect_failures, GateFailure, GateSummary};
use inventory::{PythonInventory, ScanIntegrity};

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
    functions: crate::functions::FunctionsResult,
    scan_integrity: ScanIntegrity,
    gates: GateSummary,
}

pub(super) fn run_comprehensive_deslop<W: Write>(
    request: &ComprehensiveRequest<'_>,
    writer: &mut W,
) -> Result<i32> {
    let mut inventory =
        PythonInventory::collect(request.roots, request.excludes, request.cli.output.verbose);
    let files = &inventory.files;
    let architecture = crate::architecture::build_architecture_graph(files, request.roots);
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
    let searchability = crate::searchability::analyze_searchability_with_functions(
        files,
        &inventory.definitions.functions,
    );
    let naming = crate::naming::scanner::analyze_naming(
        &inventory.definitions.functions,
        &inventory.definitions.classes,
    );
    let todos = crate::todos::analyze_todos_files(files);
    let global_files = crate::globals::collect_files(request.roots, request.excludes);
    let global_target = request
        .roots
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("."));
    let globals = crate::globals::analyze_globals_files(&global_files, &global_target);
    let exceptions = crate::exceptions::analyze_exceptions_files(files);
    let wildcards = crate::wildcards::analyze_wildcards_files(files);
    let side_effect_files = crate::side_effects::collect_files(request.roots, request.excludes);
    let side_effects = crate::side_effects::analyze_side_effects_files(&side_effect_files);
    let singletons = crate::singletons::analyze_singletons_files(files);
    let anti_patterns = crate::anti_patterns::analyze_anti_patterns_files(files);
    let include_tests = request.cli.include.include_tests
        || request.config.cytoscnpy.include_tests.unwrap_or(false);
    let mut duplicates = crate::duplicates::analyze_duplicates_files(
        files,
        &crate::duplicates::DuplicatesOptions {
            include_tests,
            ..crate::duplicates::DuplicatesOptions::default()
        },
    );
    duplicates.roots = request.roots.to_vec();
    let mut unreferenced = crate::unreferenced::analyze_unreferenced_files(
        files,
        &crate::unreferenced::UnreferencedOptions {
            include_tests,
            ..crate::unreferenced::UnreferencedOptions::default()
        },
    );
    unreferenced.roots = request.roots.to_vec();
    let functions = crate::functions::scan_files(files);
    inventory.check_additional(global_files.into_iter().chain(side_effect_files));

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
        functions: &functions,
    };
    let max_score = request
        .args
        .max_score
        .or(if request.cli.output.fail_on_any || request.args.ci {
            request.config.cytoscnpy.deslop.max_slop_index
        } else {
            None
        })
        .or(if request.args.ci { Some(40) } else { None });
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

    let mut failures = collect_failures(
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
        &functions,
        &scoring,
        request.config,
        request.cli.output.fail_on_any,
        max_score,
    );
    if !inventory.integrity.complete {
        failures.push(GateFailure {
            check: "scan_integrity",
            actual: inventory.integrity.issues.len().to_string(),
            limit: "0 skipped or unparsable files".to_owned(),
        });
    }
    let exit_code = i32::from(!failures.is_empty());
    let human_output = if request.cli.output.json {
        None
    } else if request.args.format.as_deref() == Some("llm") {
        let mut output = Vec::new();
        crate::scoring::reporter::print_llm_report(&scoring, &mut output)?;
        Some(String::from_utf8(output)?)
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
            &functions,
            &scoring,
            &failures,
            &inventory.integrity,
            request.cli.output.verbose,
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
        functions,
        scan_integrity: inventory.integrity,
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
    let cfg = crate::doctor::DoctorConfig {
        excludes: request.excludes.to_vec(),
        verbose: request.cli.output.verbose,
        fail_on_missing: false,
    };
    request
        .roots
        .iter()
        .map(|p| crate::doctor::resolve_doctor_target(p, request.analysis_root))
        .filter(|p| seen.insert(p.clone()))
        .map(|p| crate::doctor::run_doctor(&p, &cfg))
        .collect()
}
