//! Handler for the `searchability` codebase searchability and name collision subcommand.

use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::cli::PathArgs;
use crate::entry_point::paths::{
    prepare_output_path, resolve_subcommand_paths, validate_path_args,
};
use crate::searchability::{analyze_searchability, print_json_report, print_terminal_report};

/// CLI flags specific to the `searchability` command.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SearchabilityFlags {
    /// Whether to output JSON format.
    pub json: bool,
    /// Whether to fail if any function collisions are detected.
    pub fail_on_collisions: bool,
    /// Whether to fail if any duplicate filenames are detected.
    pub fail_on_duplicates: bool,
    /// Whether to fail if any searchability issue is detected.
    pub fail_on_any: bool,
    /// Verbose output mode.
    pub verbose: bool,
}

/// Executes the `searchability` subcommand.
pub(crate) fn handle_searchability<W: Write>(
    paths: &PathArgs,
    flags: SearchabilityFlags,
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

    let result = analyze_searchability(&targets, &excludes, flags.verbose);

    let has_collision_failure = (flags.fail_on_collisions || flags.fail_on_any)
        && result.stats.function_name_collisions > 0;
    let has_duplicate_failure =
        (flags.fail_on_duplicates || flags.fail_on_any) && result.stats.duplicate_filenames > 0;

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

    if has_duplicate_failure && !flags.json {
        writeln!(
            writer,
            "\n[GATE] Duplicate filenames: {} duplicate file(s) found - FAILED",
            result.stats.duplicate_filenames
        )?;
    }
    if has_collision_failure && !flags.json {
        writeln!(
            writer,
            "\n[GATE] Function name collisions: {} colliding function(s) found - FAILED",
            result.stats.function_name_collisions
        )?;
    }

    if has_collision_failure || has_duplicate_failure {
        Ok(1)
    } else {
        Ok(0)
    }
}
