//! Handler for the `functions` metric extraction subcommand.

use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::cli::PathArgs;
use crate::entry_point::paths::{
    prepare_output_path, resolve_subcommand_paths, validate_path_args,
};
use crate::functions::{
    analyze_functions, print_json_report, print_terminal_report, FunctionsResult,
};

/// CLI flags specific to the `functions` command.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FunctionsFlags {
    /// Whether to output JSON format.
    pub json: bool,
    /// Whether to exit with code 1 when any threshold is violated.
    pub fail_on_any: bool,
    /// Minimum complexity filter for output.
    pub min_complexity: Option<usize>,
    /// Minimum lines filter for output.
    pub min_lines: Option<usize>,
    /// Minimum nesting filter for output.
    pub min_nesting: Option<usize>,
    /// Maximum allowed complexity gate.
    pub max_complexity: Option<usize>,
    /// Maximum allowed lines gate.
    pub max_lines: Option<usize>,
    /// Maximum allowed nesting depth gate.
    pub max_nesting: Option<usize>,
    /// Verbose output mode.
    pub verbose: bool,
}

/// Executes the `functions` subcommand.
pub(crate) fn handle_functions<W: Write>(
    paths: &PathArgs,
    flags: FunctionsFlags,
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

    let mut result = analyze_functions(&targets, &excludes, flags.verbose);

    // Apply display filters if requested
    if flags.min_complexity.is_some() || flags.min_lines.is_some() || flags.min_nesting.is_some() {
        result.functions.retain(|f| {
            if let Some(min_cc) = flags.min_complexity {
                if f.cyclomatic_complexity < min_cc {
                    return false;
                }
            }
            if let Some(min_l) = flags.min_lines {
                if f.line_count < min_l {
                    return false;
                }
            }
            if let Some(min_n) = flags.min_nesting {
                if f.max_nesting < min_n {
                    return false;
                }
            }
            true
        });
    }

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

fn determine_failure(result: &FunctionsResult, flags: &FunctionsFlags) -> bool {
    if flags.fail_on_any {
        // Standard quality thresholds: CC <= 10, lines <= 50, nesting <= 3
        if result
            .functions
            .iter()
            .any(|f| f.cyclomatic_complexity > 10 || f.line_count > 50 || f.max_nesting > 3)
        {
            return true;
        }
    }
    if let Some(max_cc) = flags.max_complexity {
        if result.stats.max_complexity > max_cc {
            return true;
        }
    }
    if let Some(max_l) = flags.max_lines {
        if result.stats.max_lines > max_l {
            return true;
        }
    }
    if let Some(max_n) = flags.max_nesting {
        if result.stats.max_nesting > max_n {
            return true;
        }
    }
    false
}
