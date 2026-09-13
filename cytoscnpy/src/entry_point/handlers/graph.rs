//! Handler for the `graph` architecture and circular dependency subcommand.

use anyhow::Result;
use std::fs;
use std::io::Write;

use crate::architecture::{analyze_architecture, print_json_report, print_terminal_report};
use crate::cli::PathArgs;
use crate::entry_point::paths::{
    merge_excludes, prepare_output_path, resolve_subcommand_paths, validate_path_args,
};

/// CLI flags specific to the `graph` command.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GraphFlags {
    pub json: bool,
    pub cycles_only: bool,
    pub fail_on_cycles: bool,
    pub fail_on_god_modules: bool,
    pub verbose: bool,
}

/// Executes the `graph` subcommand.
pub(crate) fn handle_graph<W: Write>(
    paths: &PathArgs,
    flags: GraphFlags,
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

    let result = analyze_architecture(&effective_paths, &excludes, flags.verbose);

    if let Some(file_path) = output_file {
        let mut file = fs::File::create(&file_path)?;
        if flags.json {
            print_json_report(&result, &mut file)?;
        } else {
            print_terminal_report(&result, flags.cycles_only, &mut file)?;
        }
    } else if flags.json {
        print_json_report(&result, writer)?;
    } else {
        print_terminal_report(&result, flags.cycles_only, writer)?;
    }

    if flags.fail_on_cycles && result.stats.circular_dependency_count > 0 {
        if !flags.json {
            writeln!(
                writer,
                "\n[GATE] Circular dependencies: {} cycle(s) found - FAILED",
                result.stats.circular_dependency_count
            )?;
        }
        return Ok(1);
    }

    if flags.fail_on_god_modules && !result.stats.god_modules.is_empty() {
        if !flags.json {
            writeln!(
                writer,
                "\n[GATE] God modules: {} module(s) found - FAILED",
                result.stats.god_modules.len()
            )?;
        }
        return Ok(1);
    }

    Ok(0)
}
