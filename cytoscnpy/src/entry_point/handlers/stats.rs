use anyhow::Result;

use crate::entry_point::paths::{
    merge_excludes, prepare_output_path, resolve_subcommand_paths, validate_path_args,
};

pub(crate) fn handle_stats<W: std::io::Write>(
    paths: &crate::cli::PathArgs,
    options: crate::commands::ScanOptions,
    output: Option<String>,
    exclude: Vec<String>,
    exclude_folders: &[String],
    include_folders: &[String],
    analysis_root: &std::path::Path,
    include_tests: bool,
    verbose: bool,
    fail_on_quality: bool,
    config: crate::config::Config,
    writer: &mut W,
) -> Result<i32> {
    if let Err(code) = validate_path_args(paths) {
        return Ok(code);
    }
    // Use --root if provided, otherwise use positional paths
    let effective_paths = match resolve_subcommand_paths(paths.paths.clone(), paths.root.clone()) {
        Ok(p) => p,
        Err(code) => return Ok(code),
    };
    let excludes = merge_excludes(exclude, exclude_folders);
    let output_file = prepare_output_path(output, analysis_root)?;

    let quality_count = crate::commands::run_stats_v2(
        if paths.paths.is_empty() {
            match paths.root {
                Some(ref p) => p,
                None => analysis_root,
            }
        } else {
            &paths.paths[0]
        },
        &effective_paths,
        options,
        output_file,
        &excludes,
        include_tests,
        include_folders,
        verbose,
        config,
        writer,
    )?;

    // Quality gate check (--fail-on-quality) for stats subcommand
    if fail_on_quality && quality_count > 0 {
        if !options.json {
            eprintln!("\n[GATE] Quality issues: {quality_count} found - FAILED");
        }
        return Ok(1);
    }
    Ok(0)
}

pub(crate) fn handle_files<W: std::io::Write>(
    args: crate::cli::FilesArgs,
    exclude_folders: &[String],
    verbose: bool,
    writer: &mut W,
) -> Result<i32> {
    if let Err(code) = validate_path_args(&args.paths) {
        return Ok(code);
    }
    let paths = match resolve_subcommand_paths(args.paths.paths, args.paths.root) {
        Ok(p) => p,
        Err(code) => return Ok(code),
    };
    let exclude = merge_excludes(args.exclude, exclude_folders);
    crate::commands::run_files(&paths, args.json, &exclude, verbose, writer)?;
    Ok(0)
}
