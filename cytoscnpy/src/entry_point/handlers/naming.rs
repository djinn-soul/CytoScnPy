//! Handler for the `naming` style distribution and consistency subcommand.

use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::cli::PathArgs;
use crate::entry_point::paths::{
    prepare_output_path, resolve_subcommand_paths, validate_path_args,
};
use crate::naming::{analyze_naming, print_json_report, print_terminal_report};

/// CLI flags specific to the `naming` command.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NamingFlags {
    /// Whether to output JSON format.
    pub json: bool,
    /// Minimum consistency ratio required (0.0 - 1.0 or 0 - 100).
    pub min_consistency: Option<f64>,
    /// Whether to fail if consistency falls below threshold.
    pub fail_on_inconsistent: bool,
    /// Verbose output mode.
    pub verbose: bool,
}

/// Executes the `naming` subcommand.
pub(crate) fn handle_naming<W: Write>(
    paths: &PathArgs,
    flags: NamingFlags,
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

    let result = analyze_naming(&targets, &excludes, flags.verbose);

    let raw_threshold = flags.min_consistency.unwrap_or(0.85);
    let threshold = crate::naming::types::normalize_consistency_threshold(raw_threshold);

    let has_failure = flags.fail_on_inconsistent && !result.is_consistent(threshold);

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
            "\n[GATE] Naming consistency: {:.1}% (dominant style: {}) is below required threshold {:.1}% - FAILED",
            result.stats.consistency_score(),
            result.stats.dominant_style,
            threshold * 100.0
        )?;
    }

    if has_failure {
        Ok(1)
    } else {
        Ok(0)
    }
}
