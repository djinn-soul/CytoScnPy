use crate::cli::{Cli, Commands};
use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use crate::entry_point::config::{is_vscode_client, resolve_scan_flag, setup_configuration};
use crate::entry_point::handlers::{
    handle_analysis, handle_cc, handle_files, handle_hal, handle_mi, handle_raw, handle_stats,
    CcFlags, MiFlags,
};
use crate::entry_point::paths::{
    collect_all_target_paths, resolve_analysis_context, validate_path_args,
};
use crate::settings;

/// Runs the analyzer with the given arguments using stdout as the writer.
///
/// # Errors
///
/// Returns an error if argument parsing fails, or if the command execution fails.
pub fn run_with_args(args: Vec<String>) -> Result<i32> {
    run_with_args_to(args, &mut std::io::stdout())
}

/// Run CytoScnPy with the given arguments, writing output to the specified writer.
///
/// This is the testable version of `run_with_args` that allows output capture.
///
/// # Errors
///
/// Returns an error if argument parsing fails, or if the command execution fails.
#[allow(clippy::too_many_lines)]
pub fn run_with_args_to<W: std::io::Write>(args: Vec<String>, writer: &mut W) -> Result<i32> {
    let mut program_args = vec!["cytoscnpy".to_owned()];
    program_args.extend(args);
    let cli_var = match Cli::try_parse_from(program_args) {
        Ok(c) => c,
        Err(e) => {
            match e.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                    // Let clap print help/version as intended, but captured by redirect
                    write!(writer, "{e}")?;
                    writer.flush()?; // Flush to ensure output is visible (required for pytest)
                    return Ok(0);
                }
                _ => {
                    eprint!("{e}");
                    return Ok(1);
                }
            }
        }
    };

    // Explicit runtime validation for mutual exclusivity of --root and positional paths
    if let Err(code) = validate_path_args(&cli_var.paths) {
        return Ok(code);
    }

    let all_target_paths = collect_all_target_paths(&cli_var);
    let (effective_paths, analysis_root) = resolve_analysis_context(&cli_var, &all_target_paths);

    let app_config = setup_configuration(&effective_paths, &cli_var);
    let config = app_config.config;
    if let Err(err) = settings::initialize(config.clone()) {
        if err != crate::settings::SettingsError::AlreadyInitialized {
            return Err(err.into());
        }
    }
    let exclude_folders = app_config.exclude_folders;
    let include_folders = app_config.include_folders;
    let include_tests = app_config.include_tests;
    let is_vscode_client = is_vscode_client(&cli_var);

    // Print deprecation warning if old keys are used in config
    if config.cytoscnpy.uses_deprecated_keys() && !cli_var.output.json {
        eprintln!(
            "{}",
            "WARNING: 'complexity' and 'nesting' are deprecated in configuration. Please use 'max_complexity' and 'max_nesting' instead."
                .yellow()
                .bold()
        );
    }

    if cli_var.output.verbose && !cli_var.output.json {
        eprintln!("[VERBOSE] CytoScnPy v{}", env!("CARGO_PKG_VERSION"));
        eprintln!("[VERBOSE] Using {} threads", rayon::current_num_threads());
        if let Some(ref command) = cli_var.command {
            eprintln!("[VERBOSE] Executing subcommand: {command:?}");
        }
        eprintln!("[VERBOSE] Global Excludes: {exclude_folders:?}");
        eprintln!();
    }

    if let Some(command) = cli_var.command {
        match command {
            Commands::Raw { common, summary } => handle_raw(
                common,
                summary,
                &exclude_folders,
                &analysis_root,
                cli_var.output.verbose,
                writer,
            ),
            Commands::Cc {
                common,
                rank,
                average,
                total_average,
                show_complexity,
                order,
                no_assert,
                xml,
                fail_threshold,
            } => handle_cc(
                common,
                rank,
                CcFlags {
                    average,
                    total_average,
                    show_complexity,
                    order,
                    no_assert,
                    xml,
                    fail_threshold,
                },
                &exclude_folders,
                &analysis_root,
                cli_var.output.verbose,
                writer,
            ),
            Commands::Hal { common, functions } => handle_hal(
                common,
                functions,
                &exclude_folders,
                &analysis_root,
                cli_var.output.verbose,
                writer,
            ),
            Commands::Mi {
                common,
                rank,
                multi,
                show,
                average,
                fail_threshold,
            } => handle_mi(
                common,
                rank,
                MiFlags {
                    multi,
                    show_hooks: show,
                    average,
                    fail_threshold,
                },
                &exclude_folders,
                &analysis_root,
                cli_var.output.verbose,
                writer,
            ),
            Commands::McpServer => {
                eprintln!("Error: mcp-server command should be handled by cytoscnpy-cli directly.");
                eprintln!("If you're seeing this, please use the cytoscnpy-cli binary.");
                Ok(1)
            }
            Commands::Stats {
                paths,
                all,
                secrets,
                danger,
                quality,
                json,
                output,
                exclude,
            } => handle_stats(
                &paths,
                crate::commands::ScanOptions {
                    all,
                    inspections: crate::commands::Inspections {
                        secrets: resolve_scan_flag(
                            secrets,
                            config.cytoscnpy.secrets,
                            is_vscode_client,
                        ),
                        danger: resolve_scan_flag(
                            danger,
                            config.cytoscnpy.danger,
                            is_vscode_client,
                        ),
                        quality: resolve_scan_flag(
                            quality,
                            config.cytoscnpy.quality,
                            is_vscode_client,
                        ),
                    },
                    json,
                },
                output,
                exclude,
                &exclude_folders,
                &include_folders,
                &analysis_root,
                include_tests,
                cli_var.output.verbose,
                cli_var.output.fail_on_quality,
                config,
                writer,
            ),
            Commands::Files { args } => {
                handle_files(args, &exclude_folders, cli_var.output.verbose, writer)
            }
            Commands::Init => {
                crate::commands::run_init_in(&analysis_root, writer)?;
                Ok(0)
            }
        }
    } else {
        handle_analysis(
            &effective_paths,
            &analysis_root,
            &cli_var,
            &config,
            &exclude_folders,
            &include_folders,
            writer,
        )
    }
}
