mod metrics;
mod quality;

use super::config::resolve_scan_flag;
use super::handlers::{
    handle_context, handle_deps, handle_deslop, handle_doctor, handle_files, handle_graph,
    handle_stats, ContextFlags, DepsCliArgs, DepsFlags, DoctorFlags, GraphFlags,
};
use super::run::RuntimeContext;
use crate::cli::Commands;
use anyhow::Result;

pub(super) fn run_subcommand<W: std::io::Write>(
    command: Commands,
    cli: &crate::cli::Cli,
    context: &RuntimeContext,
    writer: &mut W,
) -> Result<i32> {
    let verbose = cli.output.verbose;
    let root_fail_on_any = cli.output.fail_on_any;
    let root_json = cli.output.json;

    match command {
        Commands::Raw { .. } | Commands::Cc { .. } | Commands::Hal { .. } | Commands::Mi { .. } => {
            metrics::handle_metric_command(command, context, verbose, writer)
        }
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
                        context.config.cytoscnpy.secrets,
                        context.is_vscode_client,
                    ),
                    danger: resolve_scan_flag(
                        danger,
                        context.config.cytoscnpy.danger,
                        context.is_vscode_client,
                    ),
                    quality: resolve_scan_flag(
                        quality,
                        context.config.cytoscnpy.quality,
                        context.is_vscode_client,
                    ),
                },
                json,
            },
            output,
            exclude,
            &context.exclude_folders,
            &context.include_folders,
            &context.analysis_root,
            context.include_tests,
            verbose,
            cli.output.fail_on_quality,
            context.config.clone(),
            writer,
        ),
        Commands::Deslop { args } => handle_deslop(&args, cli, context, writer),
        Commands::Files { args } => handle_files(
            args,
            &context.exclude_folders,
            context.include_tests,
            verbose,
            writer,
        ),
        Commands::Deps { args } => handle_deps(
            DepsCliArgs {
                effective_paths: context.effective_paths.clone(),
                flags: DepsFlags {
                    json: root_json || args.json,
                    verbose,
                    show_extra: args.extra_installed,
                    show_orphans: args.orphans,
                    fail_on_any: root_fail_on_any || args.fail_on_any,
                    fail_on_unused: args.fail_on_unused,
                    fail_on_missing: args.fail_on_missing,
                    fail_on_extra_installed: args.fail_on_extra_installed,
                    fail_on_orphans: args.fail_on_orphans,
                    include_dev_unused: args.include_dev_unused,
                },
                requirements: args.requirements,
                ignore_unused: args.ignore_unused,
                ignore_missing: args.ignore_missing,
                exclude: args.exclude,
                output_file: args.output_file,
                cli_exclude_folders: context.exclude_folders.clone(),
                impact_package: args.impact,
                venv: args.venv,
                lockfile: args.lockfile,
            },
            &context.config,
            writer,
        ),
        Commands::Searchability { .. }
        | Commands::Naming { .. }
        | Commands::Todos { .. }
        | Commands::Globals { .. }
        | Commands::Exceptions { .. }
        | Commands::Wildcards { .. }
        | Commands::SideEffects { .. }
        | Commands::Singletons { .. }
        | Commands::AntiPatterns { .. }
        | Commands::Duplicates { .. }
        | Commands::Unreferenced { .. }
        | Commands::Score { .. }
        | Commands::Functions { .. } => quality::handle_quality_command(
            command,
            root_json,
            root_fail_on_any,
            verbose,
            context,
            writer,
        ),
        Commands::Init => {
            crate::commands::run_init_in(&context.analysis_root, writer)?;
            Ok(0)
        }
        Commands::Graph { args } => handle_graph(
            &args.paths,
            GraphFlags {
                json: args.json,
                cycles_only: args.cycles_only,
                fail_on_cycles: args.fail_on_cycles,
                fail_on_god_modules: args.fail_on_god_modules,
                verbose,
            },
            args.output_file,
            args.exclude,
            &context.exclude_folders,
            &context.analysis_root,
            writer,
        ),
        Commands::Context { args } => handle_context(
            &args.paths,
            ContextFlags {
                git_months: args.git_months,
                context_budget: args.context_budget.unwrap_or(176_000),
                no_git: args.no_git,
                json: args.json,
                hotspots_only: args.hotspots_only,
                fail_on_hotspots: args.fail_on_hotspots,
                verbose,
            },
            args.output_file,
            args.exclude,
            &context.exclude_folders,
            &context.analysis_root,
            writer,
        ),
        Commands::Doctor { args } => handle_doctor(
            &args.paths,
            DoctorFlags {
                json: args.json,
                fail_on_missing: args.fail_on_missing,
                verbose,
            },
            args.output_file,
            args.exclude,
            &context.exclude_folders,
            &context.analysis_root,
            writer,
        ),
    }
}
