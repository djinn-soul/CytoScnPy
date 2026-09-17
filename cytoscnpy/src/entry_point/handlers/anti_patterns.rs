//! Handler for the `anti-patterns` detection subcommand.

use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::anti_patterns::{
    analyze_anti_patterns, print_json_report, print_terminal_report, AntiPatternsResult,
};
use crate::cli::PathArgs;
use crate::entry_point::paths::{
    prepare_output_path, resolve_subcommand_paths, validate_path_args,
};

/// CLI flags specific to the `anti-patterns` command.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AntiPatternsFlags {
    /// Whether to output JSON format.
    pub json: bool,
    /// Whether to exit with code 1 when any anti-pattern is found.
    pub fail_on_any: bool,
    /// Optional upper limit for total anti-patterns.
    pub max_anti_patterns: Option<usize>,
    /// Optional upper limit for magic numbers.
    pub max_magic_numbers: Option<usize>,
    /// Optional upper limit for nested callbacks.
    pub max_nested_callbacks: Option<usize>,
    /// Verbose output mode.
    pub verbose: bool,
}

/// Executes the `anti-patterns` subcommand.
pub(crate) fn handle_anti_patterns<W: Write>(
    paths: &PathArgs,
    flags: AntiPatternsFlags,
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

    let result = analyze_anti_patterns(&targets, &excludes, flags.verbose);

    let has_failure = determine_failure(&result, &flags);

    let base = if targets.len() == 1 && targets[0].is_dir() {
        Some(targets[0].as_path())
    } else {
        Some(analysis_root)
    };

    if flags.json {
        print_json_report(&result, writer)?;
    } else {
        print_terminal_report(&result, base, writer)?;
    }

    if let Some(path) = output_file {
        let mut buffer = Vec::new();
        if flags.json {
            print_json_report(&result, &mut buffer)?;
        } else {
            print_terminal_report(&result, base, &mut buffer)?;
        }
        fs::write(&path, buffer)?;
    }

    if has_failure {
        Ok(1)
    } else {
        Ok(0)
    }
}

fn determine_failure(result: &AntiPatternsResult, flags: &AntiPatternsFlags) -> bool {
    if flags.fail_on_any && !result.is_clean() {
        return true;
    }
    if let Some(limit) = flags.max_anti_patterns {
        if result.stats.total > limit {
            return true;
        }
    }
    if let Some(limit) = flags.max_magic_numbers {
        if result.stats.magic_numbers > limit {
            return true;
        }
    }
    if let Some(limit) = flags.max_nested_callbacks {
        if result.stats.nested_callbacks > limit {
            return true;
        }
    }
    false
}
