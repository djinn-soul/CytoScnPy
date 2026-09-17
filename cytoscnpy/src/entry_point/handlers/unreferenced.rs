//! Handler for the `unreferenced` large functions in isolated files subcommand.

use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::cli::PathArgs;
use crate::entry_point::paths::{
    prepare_output_path, resolve_subcommand_paths, validate_path_args,
};
use crate::unreferenced::{
    analyze_unreferenced, print_json_report, print_terminal_report, UnreferencedOptions,
    UnreferencedResult,
};

/// CLI flags specific to the `unreferenced` command.
#[derive(Debug, Clone, Copy)]
pub(crate) struct UnreferencedFlags {
    /// Whether to output JSON format.
    pub json: bool,
    /// Whether to exit with code 1 when any unreferenced function is found.
    pub fail_on_any: bool,
    /// Optional limit on unreferenced function count.
    pub max_unreferenced: Option<usize>,
    /// Optional limit on total unreferenced lines.
    pub max_unreferenced_lines: Option<usize>,
    /// Minimum physical lines for large functions.
    pub min_lines: Option<usize>,
    /// Include test files in analysis.
    pub include_tests: bool,
    /// Verbose output mode.
    pub verbose: bool,
}

/// Executes the `unreferenced` subcommand.
pub(crate) fn handle_unreferenced<W: Write>(
    paths: &PathArgs,
    flags: UnreferencedFlags,
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

    let mut options = UnreferencedOptions::default();
    if let Some(ml) = flags.min_lines {
        options.min_lines = ml;
    }
    options.include_tests = flags.include_tests;

    let result = analyze_unreferenced(&targets, &excludes, &options, flags.verbose);
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
            "\n[GATE] {} unreferenced large function(s) ({} lines) in isolated files detected - FAILED",
            result.stats.total_unreferenced_functions,
            result.stats.total_unreferenced_lines,
        )?;
    }

    if has_failure {
        Ok(1)
    } else {
        Ok(0)
    }
}

fn determine_failure(result: &UnreferencedResult, flags: &UnreferencedFlags) -> bool {
    if flags.fail_on_any && !result.is_clean() {
        return true;
    }
    if let Some(limit) = flags.max_unreferenced {
        if result.stats.total_unreferenced_functions > limit {
            return true;
        }
    }
    if let Some(limit) = flags.max_unreferenced_lines {
        if result.stats.total_unreferenced_lines > limit {
            return true;
        }
    }
    false
}
