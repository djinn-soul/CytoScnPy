use anyhow::Result;

mod unified;

use crate::entry_point::paths::{merge_excludes, resolve_subcommand_paths, validate_path_args};

pub(crate) fn handle_deslop<W: std::io::Write>(
    args: &crate::cli::DeslopArgs,
    cli: &crate::cli::Cli,
    runtime: &crate::entry_point::run::RuntimeContext,
    writer: &mut W,
) -> Result<i32> {
    if let Err(code) = validate_path_args(&args.paths) {
        return Ok(code);
    }
    let roots = match resolve_subcommand_paths(args.paths.paths.clone(), args.paths.root.clone()) {
        Ok(paths) => paths,
        Err(code) => return Ok(code),
    };
    let mut all_excludes = args.exclude.clone();
    all_excludes.extend(args.ignore.clone());
    let excludes = merge_excludes(all_excludes, &runtime.exclude_folders);

    unified::run_comprehensive_deslop(
        &unified::ComprehensiveRequest {
            roots: &roots,
            analysis_root: &runtime.analysis_root,
            excludes: &excludes,
            args,
            cli,
            config: &runtime.config,
        },
        writer,
    )
}
