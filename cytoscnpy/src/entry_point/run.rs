use crate::cli::Cli;
use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use crate::entry_point::config::{is_vscode_client, setup_configuration};
use crate::entry_point::handlers::handle_analysis;
use crate::entry_point::paths::{
    collect_all_target_paths, resolve_analysis_context, validate_path_args,
};
use crate::settings;
use std::path::PathBuf;

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
pub fn run_with_args_to<W: std::io::Write>(args: Vec<String>, writer: &mut W) -> Result<i32> {
    let cli_var = match parse_cli_or_exit(args, writer)? {
        Ok(cli) => cli,
        Err(code) => return Ok(code),
    };

    // Explicit runtime validation for mutual exclusivity of --root and positional paths
    if let Err(code) = validate_path_args(&cli_var.paths) {
        return Ok(code);
    }

    let context = build_runtime_context(&cli_var)?;
    if let Err(err) = settings::initialize(context.config.clone()) {
        if err != crate::settings::SettingsError::AlreadyInitialized {
            return Err(err.into());
        }
    }

    print_runtime_messages(&cli_var, &context);

    if let Some(command) = cli_var.command {
        super::subcommands::run_subcommand(
            command,
            cli_var.output.verbose,
            cli_var.output.fail_on_quality,
            cli_var.output.fail_on_any,
            &context,
            writer,
        )
    } else {
        handle_analysis(
            &context.effective_paths,
            &context.analysis_root,
            &cli_var,
            &context.config,
            &context.exclude_folders,
            &context.include_folders,
            writer,
        )
    }
}

fn parse_cli_or_exit<W: std::io::Write>(
    args: Vec<String>,
    writer: &mut W,
) -> Result<std::result::Result<Cli, i32>> {
    let mut program_args = vec!["cytoscnpy".to_owned()];
    program_args.extend(args);
    match Cli::try_parse_from(program_args) {
        Ok(cli) => Ok(Ok(cli)),
        Err(error) => match error.kind() {
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                write!(writer, "{error}")?;
                writer.flush()?;
                Ok(Err(0))
            }
            _ => {
                eprint!("{error}");
                Ok(Err(1))
            }
        },
    }
}

pub(super) struct RuntimeContext {
    pub(super) effective_paths: Vec<PathBuf>,
    pub(super) analysis_root: PathBuf,
    pub(super) config: crate::config::Config,
    pub(super) exclude_folders: Vec<String>,
    pub(super) include_folders: Vec<String>,
    pub(super) include_tests: bool,
    pub(super) is_vscode_client: bool,
}

fn build_runtime_context(cli_var: &Cli) -> Result<RuntimeContext> {
    let all_target_paths = collect_all_target_paths(cli_var);
    let (effective_paths, analysis_root) = resolve_analysis_context(cli_var, &all_target_paths);
    let app_config = setup_configuration(&effective_paths, cli_var)?;

    Ok(RuntimeContext {
        effective_paths,
        analysis_root,
        exclude_folders: app_config.exclude_folders,
        include_folders: app_config.include_folders,
        include_tests: app_config.include_tests,
        is_vscode_client: is_vscode_client(cli_var),
        config: app_config.config,
    })
}

fn print_runtime_messages(cli_var: &Cli, context: &RuntimeContext) {
    if context.config.cytoscnpy.uses_deprecated_keys() && !cli_var.output.json {
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
        eprintln!("[VERBOSE] Global Excludes: {:?}", context.exclude_folders);
        eprintln!();
    }
}
