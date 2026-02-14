mod context;
mod fix;
mod gates;
mod report;
mod run;

use anyhow::Result;

use context::build_analysis_context;
use fix::run_fix_if_requested;
use gates::apply_gates;
use report::report_results;
use run::run_analysis;

pub(crate) fn handle_analysis<W: std::io::Write>(
    effective_paths: &[std::path::PathBuf],
    analysis_root: &std::path::Path,
    cli_var: &crate::cli::Cli,
    config: &crate::config::Config,
    base_exclude_folders: &[String],
    base_include_folders: &[String],
    writer: &mut W,
) -> Result<i32> {
    // Check for non-existent paths early
    for path in effective_paths {
        if !path.exists() {
            if !cli_var.output.json {
                eprintln!("Error: Path does not exist: {}", path.display());
            }
            return Ok(1);
        }
    }

    let is_vscode_client = crate::entry_point::config::is_vscode_client(cli_var);
    let context = build_analysis_context(
        cli_var,
        config,
        is_vscode_client,
        base_exclude_folders,
        base_include_folders,
    );

    let mut run = run_analysis(
        effective_paths,
        analysis_root,
        cli_var,
        config,
        &context,
        writer,
    )?;

    // If --no-dead flag is set, clear dead code detection results
    // (only show security/quality scans)
    if cli_var.scan.no_dead {
        run.result.unused_functions.clear();
        run.result.unused_methods.clear();
        run.result.unused_classes.clear();
        run.result.unused_imports.clear();
        run.result.unused_variables.clear();
        run.result.unused_parameters.clear();
    }

    let json_fix_mode = cli_var.fix && cli_var.output.json && !cli_var.clones;
    if !json_fix_mode {
        report_results(cli_var, analysis_root, &context, &run, writer)?;
    }
    run_fix_if_requested(
        cli_var,
        analysis_root,
        context.confidence,
        &run.result,
        writer,
    )?;

    let exit_code = apply_gates(cli_var, config, analysis_root, &context, &run, writer)?;
    writer.flush()?;
    Ok(exit_code)
}
