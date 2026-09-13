//! Handler for the `context` Git churn, hotspots, and LLM budget subcommand.

use anyhow::Result;
use std::fs;
use std::io::Write;

use crate::cli::PathArgs;
use crate::context::{
    analyze_context, has_severe_hotspots, print_json_report, print_terminal_report, ContextConfig,
};
use crate::entry_point::paths::{
    merge_excludes, prepare_output_path, resolve_subcommand_paths, validate_path_args,
};

/// CLI flags specific to the `context` command.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ContextFlags {
    pub git_months: Option<u32>,
    pub context_budget: usize,
    pub no_git: bool,
    pub json: bool,
    pub hotspots_only: bool,
    pub fail_on_hotspots: bool,
    pub verbose: bool,
}

/// Executes the `context` subcommand.
pub(crate) fn handle_context<W: Write>(
    paths: &PathArgs,
    flags: ContextFlags,
    output: Option<String>,
    exclude: Vec<String>,
    exclude_folders: &[String],
    analysis_root: &std::path::Path,
    writer: &mut W,
) -> Result<i32> {
    if let Err(code) = validate_path_args(paths) {
        return Ok(code);
    }
    let effective_paths = match resolve_subcommand_paths(paths.paths.clone(), paths.root.clone()) {
        Ok(p) => p,
        Err(code) => return Ok(code),
    };
    let excludes = merge_excludes(exclude, exclude_folders);
    let output_file = prepare_output_path(output, analysis_root)?;

    let config = ContextConfig {
        git_months: flags.git_months,
        context_budget: flags.context_budget,
        no_git: flags.no_git,
        hotspots_only: flags.hotspots_only,
        fail_on_hotspots: flags.fail_on_hotspots,
        verbose: flags.verbose,
    };

    let result = analyze_context(&effective_paths, &excludes, &config);

    if let Some(file_path) = output_file {
        let mut file = fs::File::create(&file_path)?;
        if flags.json {
            print_json_report(&result, &mut file)?;
        } else {
            print_terminal_report(&result, flags.hotspots_only, &mut file)?;
        }
    } else if flags.json {
        print_json_report(&result, &mut *writer)?;
    } else {
        print_terminal_report(&result, flags.hotspots_only, &mut *writer)?;
    }

    if flags.fail_on_hotspots && has_severe_hotspots(&result.hotspots) {
        let severe_count = result
            .hotspots
            .iter()
            .filter(|h| {
                matches!(
                    h.risk_level,
                    crate::context::HotspotRiskLevel::Critical
                        | crate::context::HotspotRiskLevel::High
                )
            })
            .count();

        if !flags.json {
            writeln!(
                writer,
                "\n[GATE] High-churn complex hotspots: {severe_count} severe file(s) found - FAILED"
            )?;
        }
        return Ok(1);
    }

    Ok(0)
}
