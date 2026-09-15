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
    let targets = if effective_paths.is_empty() {
        vec![analysis_root.to_path_buf()]
    } else {
        effective_paths
    };
    let excludes = crate::entry_point::paths::merge_excludes(exclude, exclude_folders);
    let output_file = prepare_output_path(output, analysis_root)?;

    let config = DoctorConfig {
        fail_on_missing: flags.fail_on_missing,
        verbose: flags.verbose,
        excludes,
    };

    let mut any_failed = false;
    let mut total_failed_recs = 0usize;
    let mut results = Vec::with_capacity(targets.len());

    for target in &targets {
        let result = run_doctor(target, &config);
        if flags.fail_on_missing
            && matches!(
                result.reliability.verdict,
                SetupVerdict::Incomplete | SetupVerdict::AtRisk
            )
        {
            any_failed = true;
            total_failed_recs += result.reliability.recommendations.len();
        }
        results.push(result);
    }

    let write_output = |w: &mut dyn Write| -> Result<()> {
        if flags.json {
            if results.len() == 1 {
                print_json_report(&results[0], w)?;
            } else {
                serde_json::to_writer_pretty(&mut *w, &results)?;
                writeln!(w)?;
            }
        } else {
            for (idx, result) in results.iter().enumerate() {
                if targets.len() > 1 {
                    writeln!(w, "\n=== Repository Health: {} ===", targets[idx].display())?;
                }
                print_terminal_report(result, &mut *w)?;
            }
        }
        Ok(())
    };

    if let Some(file_path) = output_file {
        let mut file = fs::File::create(&file_path)?;
        write_output(&mut file)?;
    } else {
        write_output(&mut *writer)?;
    }

    if any_failed {
        if !flags.json {
            writeln!(
                writer,
                "\n[GATE] Setup reliability check failed: {total_failed_recs} recommendation(s) - FAILED"
            )?;
        }
        return Ok(1);
    }

    Ok(0)
}
