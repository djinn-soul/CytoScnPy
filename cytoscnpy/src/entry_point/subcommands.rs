use super::config::resolve_scan_flag;
use super::handlers::{
    handle_cc, handle_files, handle_hal, handle_mi, handle_raw, handle_stats, CcFlags, DepsCliArgs,
    DepsFlags, MiFlags,
};
use super::run::RuntimeContext;
use crate::cli::Commands;
use anyhow::Result;

pub(super) fn run_subcommand<W: std::io::Write>(
    command: Commands,
    verbose: bool,
    fail_on_quality: bool,
    root_fail_on_any: bool,
    context: &RuntimeContext,
    writer: &mut W,
) -> Result<i32> {
    match command {
        Commands::Raw { common, summary } => handle_raw(
            common,
            summary,
            &context.exclude_folders,
            &context.analysis_root,
            context.include_tests,
            verbose,
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
            &context.exclude_folders,
            &context.analysis_root,
            context.include_tests,
            verbose,
            writer,
        ),
        Commands::Hal { common, functions } => handle_hal(
            common,
            functions,
            &context.exclude_folders,
            &context.analysis_root,
            context.include_tests,
            verbose,
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
            &context.exclude_folders,
            &context.analysis_root,
            context.include_tests,
            verbose,
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
            fail_on_quality,
            context.config.clone(),
            writer,
        ),
        Commands::Files { args } => handle_files(
            args,
            &context.exclude_folders,
            context.include_tests,
            verbose,
            writer,
        ),
        Commands::Deps {
            paths: _,
            json,
            requirements,
            ignore_unused,
            ignore_missing,
            exclude,
            output_file,
            extra_installed,
            orphans,
            include_dev_unused,
            fail_on_any,
            fail_on_unused,
            fail_on_missing,
            fail_on_extra_installed,
            fail_on_orphans,
            impact,
            venv,
            lockfile,
        } => super::handlers::handle_deps(
            DepsCliArgs {
                effective_paths: context.effective_paths.clone(),
                flags: DepsFlags {
                    json,
                    verbose,
                    show_extra: extra_installed,
                    show_orphans: orphans,
                    fail_on_any: root_fail_on_any || fail_on_any,
                    fail_on_unused,
                    fail_on_missing,
                    fail_on_extra_installed,
                    fail_on_orphans,
                    include_dev_unused,
                },
                requirements,
                ignore_unused,
                ignore_missing,
                exclude,
                output_file,
                cli_exclude_folders: context.exclude_folders.clone(),
                impact_package: impact,
                venv,
                lockfile,
            },
            &context.config,
            writer,
        ),
        Commands::Init => {
            crate::commands::run_init_in(&context.analysis_root, writer)?;
            Ok(0)
        }
    }
}
