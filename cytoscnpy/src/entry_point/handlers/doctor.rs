//! Handler for the `doctor` / `health` repository health and setup reliability subcommand.

use anyhow::Result;
use std::fs;
use std::io::Write;

use crate::cli::PathArgs;
use crate::doctor::{
    print_json_report, print_terminal_report, run_doctor, DoctorConfig, SetupVerdict,
};
use crate::entry_point::paths::{
    prepare_output_path, resolve_subcommand_paths, validate_path_args,
};

/// CLI flags specific to the `doctor` command.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DoctorFlags {
    /// Whether to output JSON.
    pub json: bool,
    /// Whether to exit with code 1 if recommended tooling is missing or setup is fragile.
    pub fail_on_missing: bool,
    /// Verbose output mode.
    pub verbose: bool,
}

/// Executes the `doctor` subcommand.
pub(crate) fn handle_doctor<W: Write>(
    paths: &PathArgs,
    flags: DoctorFlags,
    output: Option<String>,
    _exclude: Vec<String>,
    _exclude_folders: &[String],
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
    let target_path = effective_paths
        .first()
        .map_or(analysis_root, std::convert::AsRef::as_ref);
    let output_file = prepare_output_path(output, analysis_root)?;

    let config = DoctorConfig {
        fail_on_missing: flags.fail_on_missing,
        verbose: flags.verbose,
    };

    let result = run_doctor(target_path, config);

    if let Some(file_path) = output_file {
        let mut file = fs::File::create(&file_path)?;
        if flags.json {
            print_json_report(&result, &mut file)?;
        } else {
            print_terminal_report(&result, &mut file)?;
        }
    } else if flags.json {
        print_json_report(&result, &mut *writer)?;
    } else {
        print_terminal_report(&result, &mut *writer)?;
    }

    if flags.fail_on_missing
        && matches!(
            result.reliability.verdict,
            SetupVerdict::Incomplete | SetupVerdict::AtRisk
        )
    {
        if !flags.json {
            writeln!(
                writer,
                "\n[GATE] Setup reliability check failed: {} recommendation(s) - FAILED",
                result.reliability.recommendations.len()
            )?;
        }
        return Ok(1);
    }

    Ok(0)
}
