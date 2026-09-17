//! Handler for the `side-effects` detection subcommand.

use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::cli::PathArgs;
use crate::entry_point::paths::{
    prepare_output_path, resolve_subcommand_paths, validate_path_args,
};
use crate::side_effects::{analyze_side_effects, print_json_report, print_terminal_report};

/// CLI flags specific to the `side-effects` command.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SideEffectsFlags {
    /// Whether to output JSON format.
    pub json: bool,
    /// Whether to exit with code 1 when any side effect is found.
    pub fail_on_any: bool,
    /// Optional upper limit for side effect count.
    pub max_side_effects: Option<usize>,
    /// Verbose output mode.
    pub verbose: bool,
}

/// Executes the `side-effects` subcommand.
pub(crate) fn handle_side_effects<W: Write>(
    paths: &PathArgs,
    flags: SideEffectsFlags,
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

    let result = analyze_side_effects(&targets, &excludes, flags.verbose);

    let has_failure = determine_failure(&result, &flags);

    let base = if targets.len() == 1 && targets[0].is_dir() {
        Some(targets[0].as_path())
    } else {
        Some(analysis_root)
    };

    if let Some(out_path) = output_file {
        let mut file = fs::File::create(&out_path)?;
        if flags.json {
            print_json_report(&result, &mut file)?;
        } else {
            print_terminal_report(&result, base, &mut file)?;
        }
        if !flags.json {
            writeln!(writer, "Report written to: {out_path}")?;
        }
    } else if flags.json {
        print_json_report(&result, &mut *writer)?;
    } else {
        print_terminal_report(&result, base, &mut *writer)?;
    }

    if has_failure && !flags.json {
        writeln!(
            writer,
            "\n[GATE] {} module-level side effect(s) detected - FAILED",
            result.stats.total
        )?;
    }

    if has_failure {
        Ok(1)
    } else {
        Ok(0)
    }
}

fn determine_failure(
    result: &crate::side_effects::SideEffectsResult,
    flags: &SideEffectsFlags,
) -> bool {
    if flags.fail_on_any && !result.is_clean() {
        return true;
    }
    if let Some(limit) = flags.max_side_effects {
        if result.stats.total > limit {
            return true;
        }
    }
    false
}
