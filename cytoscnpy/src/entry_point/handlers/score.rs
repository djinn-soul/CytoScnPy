//! Handler for the `score` (slop-index) subcommand.

use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::cli::PathArgs;
use crate::entry_point::paths::{
    prepare_output_path, resolve_subcommand_paths, validate_path_args,
};
use crate::scoring::{
    print_json_report, print_terminal_report, score_repository, ScoreResult, ScoringContext,
    ScoringOptions,
};

/// CLI flags specific to the `score` command.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ScoreFlags {
    /// Maximum allowed Slop Index (0-100).
    pub max_score: Option<u32>,
    /// CI mode.
    pub ci: bool,
    /// Fail if any dimension rating is > 0.
    pub fail_on_any: bool,
    /// Whether to output JSON.
    pub json: bool,
    /// Whether to output LLM markdown remediation format.
    pub llm: bool,
    /// Disable Git history analysis.
    pub no_git: bool,
    /// Git lookback in months.
    pub git_months: Option<u32>,
    /// Context capacity in tokens.
    pub context_budget: usize,
    /// Verbose output mode.
    pub verbose: bool,
}

/// Executes the `score` subcommand.
pub(crate) fn handle_score<W: Write>(
    paths: &PathArgs,
    flags: ScoreFlags,
    output: Option<String>,
    exclude: Vec<String>,
    exclude_folders: &[String],
    analysis_root: &Path,
    writer: &mut W,
) -> Result<i32> {
    if let Err(code) = validate_path_args(paths) {
        return Ok(code);
    }
    let effective_paths = match resolve_subcommand_paths(paths.paths.clone(), paths.root.clone()) {
        Ok(p) => p,
        Err(code) => return Ok(code),
    };
    let targets = if effective_paths.is_empty() {
        vec![analysis_root.to_path_buf()]
    } else {
        effective_paths
    };
    let excludes = crate::entry_point::paths::merge_excludes(exclude, exclude_folders);
    let output_file = prepare_output_path(output, analysis_root)?;

    let result = run_score_pipeline(&targets, &excludes, analysis_root, &flags);

    if let Some(out_path) = output_file {
        let mut file = fs::File::create(&out_path)?;
        if flags.json {
            print_json_report(&result, &mut file)?;
        } else if flags.llm {
            crate::scoring::reporter::print_llm_report(&result, &mut file)?;
        } else {
            print_terminal_report(&result, &mut file)?;
        }
        if !flags.json {
            writeln!(writer, "Report written to: {out_path}")?;
        }
    } else if flags.json {
        print_json_report(&result, &mut *writer)?;
    } else if flags.llm {
        crate::scoring::reporter::print_llm_report(&result, &mut *writer)?;
    } else {
        print_terminal_report(&result, &mut *writer)?;
    }

    if result.passed_gate {
        Ok(0)
    } else {
        Ok(1)
    }
}

fn run_score_pipeline(
    targets: &[std::path::PathBuf],
    excludes: &[String],
    analysis_root: &Path,
    flags: &ScoreFlags,
) -> ScoreResult {
    let architecture = crate::architecture::analyze_architecture(targets, excludes, flags.verbose);

    let context = crate::context::analyze_context(
        targets,
        excludes,
        &crate::context::ContextConfig {
            git_months: flags.git_months,
            context_budget: flags.context_budget,
            no_git: flags.no_git,
            verbose: flags.verbose,
            ..crate::context::ContextConfig::default()
        },
    );

    let doctor_results: Vec<crate::doctor::DoctorResult> = targets
        .iter()
        .map(|path| {
            let p = crate::doctor::resolve_doctor_target(path, analysis_root);
            crate::doctor::run_doctor(
                &p,
                &crate::doctor::DoctorConfig {
                    excludes: excludes.to_vec(),
                    verbose: flags.verbose,
                    fail_on_missing: false,
                },
            )
        })
        .collect();
    let doc_root = doctor_results
        .first()
        .map_or(analysis_root, |d| d.root_path.as_path());
    let doctor = crate::doctor::aggregate_doctor_results(&doctor_results, doc_root);

    let searchability =
        crate::searchability::analyze_searchability(targets, excludes, flags.verbose);
    let naming = crate::naming::analyze_naming(targets, excludes, flags.verbose);
    let todos = crate::todos::analyze_todos(targets, excludes, flags.verbose);
    let globals = crate::globals::analyze_globals(targets, excludes, flags.verbose);
    let exceptions = crate::exceptions::analyze_exceptions(targets, excludes, flags.verbose);
    let wildcards = crate::wildcards::analyze_wildcards(targets, excludes, flags.verbose);
    let side_effects = crate::side_effects::analyze_side_effects(targets, excludes, flags.verbose);
    let singletons = crate::singletons::analyze_singletons(targets, excludes, flags.verbose);
    let anti_patterns =
        crate::anti_patterns::analyze_anti_patterns(targets, excludes, flags.verbose);
    let duplicates = crate::duplicates::analyze_duplicates(
        targets,
        excludes,
        &crate::duplicates::DuplicatesOptions::default(),
        flags.verbose,
    );
    let unreferenced = crate::unreferenced::analyze_unreferenced(
        targets,
        excludes,
        &crate::unreferenced::UnreferencedOptions::default(),
        flags.verbose,
    );

    let ctx = ScoringContext {
        architecture: &architecture,
        context: &context,
        doctor: doctor.as_ref(),
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

    let options = ScoringOptions {
        max_score: flags.max_score,
        fail_on_any: flags.fail_on_any,
        ci: flags.ci,
        context_budget: flags.context_budget,
        no_git: flags.no_git,
        git_months: flags.git_months,
    };

    score_repository(&ctx, &options)
}
