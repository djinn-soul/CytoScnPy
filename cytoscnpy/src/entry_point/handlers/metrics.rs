use anyhow::Result;

use crate::entry_point::paths::{
    merge_excludes, prepare_output_path, resolve_subcommand_paths, validate_path_args,
};

#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct CcFlags {
    pub(crate) average: bool,
    pub(crate) total_average: bool,
    pub(crate) show_complexity: bool,
    pub(crate) order: Option<String>,
    pub(crate) no_assert: bool,
    pub(crate) xml: bool,
    pub(crate) fail_threshold: Option<usize>,
}

pub(crate) fn handle_raw<W: std::io::Write>(
    common: crate::cli::MetricArgs,
    summary: bool,
    exclude_folders: &[String],
    analysis_root: &std::path::Path,
    verbose: bool,
    writer: &mut W,
) -> Result<i32> {
    if let Err(code) = validate_path_args(&common.paths) {
        return Ok(code);
    }
    let paths = match resolve_subcommand_paths(common.paths.paths, common.paths.root) {
        Ok(p) => p,
        Err(code) => return Ok(code),
    };
    let exclude = merge_excludes(common.exclude, exclude_folders);
    let output_file = prepare_output_path(common.output_file, analysis_root)?;
    crate::commands::run_raw(
        &paths,
        common.json,
        exclude,
        common.ignore,
        summary,
        output_file,
        verbose,
        writer,
    )?;
    Ok(0)
}

pub(crate) fn handle_cc<W: std::io::Write>(
    common: crate::cli::MetricArgs,
    rank: crate::cli::RankArgs,
    flags: CcFlags,
    exclude_folders: &[String],
    analysis_root: &std::path::Path,
    verbose: bool,
    writer: &mut W,
) -> Result<i32> {
    if let Err(code) = validate_path_args(&common.paths) {
        return Ok(code);
    }
    let paths = match resolve_subcommand_paths(common.paths.paths, common.paths.root) {
        Ok(p) => p,
        Err(code) => return Ok(code),
    };
    let exclude = merge_excludes(common.exclude, exclude_folders);
    let output_file = prepare_output_path(common.output_file, analysis_root)?;
    crate::commands::run_cc(
        &paths,
        crate::commands::CcOptions {
            json: common.json,
            exclude,
            ignore: common.ignore,
            min_rank: rank.min_rank,
            max_rank: rank.max_rank,
            average: flags.average,
            total_average: flags.total_average,
            show_complexity: flags.show_complexity,
            order: flags.order,
            no_assert: flags.no_assert,
            xml: flags.xml,
            fail_threshold: flags.fail_threshold,
            output_file,
            verbose,
        },
        writer,
    )?;
    Ok(0)
}

pub(crate) fn handle_hal<W: std::io::Write>(
    common: crate::cli::MetricArgs,
    functions: bool,
    exclude_folders: &[String],
    analysis_root: &std::path::Path,
    verbose: bool,
    writer: &mut W,
) -> Result<i32> {
    if let Err(code) = validate_path_args(&common.paths) {
        return Ok(code);
    }
    let paths = match resolve_subcommand_paths(common.paths.paths, common.paths.root) {
        Ok(p) => p,
        Err(code) => return Ok(code),
    };
    let exclude = merge_excludes(common.exclude, exclude_folders);
    let output_file = prepare_output_path(common.output_file, analysis_root)?;
    crate::commands::run_hal(
        &paths,
        common.json,
        exclude,
        common.ignore,
        functions,
        output_file,
        verbose,
        writer,
    )?;
    Ok(0)
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MiFlags {
    pub(crate) multi: bool,
    pub(crate) show_hooks: bool,
    pub(crate) average: bool,
    pub(crate) fail_threshold: Option<f64>,
}

pub(crate) fn handle_mi<W: std::io::Write>(
    common: crate::cli::MetricArgs,
    rank: crate::cli::RankArgs,
    flags: MiFlags,
    exclude_folders: &[String],
    analysis_root: &std::path::Path,
    verbose: bool,
    writer: &mut W,
) -> Result<i32> {
    if let Err(code) = validate_path_args(&common.paths) {
        return Ok(code);
    }
    let paths = match resolve_subcommand_paths(common.paths.paths, common.paths.root) {
        Ok(p) => p,
        Err(code) => return Ok(code),
    };
    let exclude = merge_excludes(common.exclude, exclude_folders);
    let output_file = prepare_output_path(common.output_file, analysis_root)?;
    crate::commands::run_mi(
        &paths,
        crate::commands::MiOptions {
            json: common.json,
            exclude,
            ignore: common.ignore,
            min_rank: rank.min_rank,
            max_rank: rank.max_rank,
            multi: flags.multi,
            show: flags.show_hooks,
            average: flags.average,
            fail_threshold: flags.fail_threshold,
            output_file,
            verbose,
        },
        writer,
    )?;
    Ok(0)
}
