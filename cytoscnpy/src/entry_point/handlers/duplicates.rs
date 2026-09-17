//! Handler for the `duplicates` code clusters and line totals subcommand.

use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::cli::PathArgs;
use crate::duplicates::{
    analyze_duplicates, print_json_report, print_terminal_report, DuplicatesOptions,
    DuplicatesResult,
};
use crate::entry_point::paths::{
    prepare_output_path, resolve_subcommand_paths, validate_path_args,
};

/// CLI flags specific to the `duplicates` command.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DuplicatesFlags {
    /// Whether to output JSON format.
    pub json: bool,
    /// Whether to exit with code 1 when any duplicate cluster is found.
    pub fail_on_any: bool,
    /// Optional limit on duplicate cluster count.
    pub max_clusters: Option<usize>,
    /// Optional limit on total non-overlapping duplicate lines.
    pub max_duplicate_lines: Option<usize>,
    /// Optional limit on duplication percentage.
    pub max_duplicate_pct: Option<f64>,
    /// Minimum similarity threshold.
    pub min_similarity: Option<f64>,
    /// Minimum line threshold.
    pub min_lines: Option<usize>,
    /// Include test files in duplication analysis.
    pub include_tests: bool,
    /// Verbose output mode.
    pub verbose: bool,
}

/// Executes the `duplicates` subcommand.
pub(crate) fn handle_duplicates<W: Write>(
    paths: &PathArgs,
    flags: DuplicatesFlags,
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

    let mut options = DuplicatesOptions::default();
    if let Some(s) = flags.min_similarity {
        options.min_similarity = s;
    }
    if let Some(l) = flags.min_lines {
        options.min_lines = l;
    }
    options.include_tests = flags.include_tests;

    let result = analyze_duplicates(&targets, &excludes, &options, flags.verbose);
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
            "\n[GATE] Duplicate code threshold exceeded ({} clusters, {} lines, {:.1}%) - FAILED",
            result.stats.cluster_count,
            result.stats.total_duplicate_lines,
            result.stats.duplicate_pct,
        )?;
    }

    if has_failure {
        Ok(1)
    } else {
        Ok(0)
    }
}

fn determine_failure(result: &DuplicatesResult, flags: &DuplicatesFlags) -> bool {
    if flags.fail_on_any && !result.is_clean() {
        return true;
    }
    if let Some(limit) = flags.max_clusters {
        if result.stats.cluster_count > limit {
            return true;
        }
    }
    if let Some(limit) = flags.max_duplicate_lines {
        if result.stats.total_duplicate_lines > limit {
            return true;
        }
    }
    if let Some(limit) = flags.max_duplicate_pct {
        if result.stats.duplicate_pct > limit {
            return true;
        }
    }
    false
}
