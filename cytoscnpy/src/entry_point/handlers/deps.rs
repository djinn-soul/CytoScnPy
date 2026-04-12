use crate::config::Config;
use anyhow::Result;
use std::fs::File;
use std::path::PathBuf;

/// Boolean flag group for the `deps` subcommand.
#[derive(Clone, Copy)]
pub(crate) struct DepsFlags {
    pub json: bool,
    pub verbose: bool,
    pub show_extra: bool,
    pub show_orphans: bool,
}

/// All CLI-provided arguments for the `deps` subcommand, grouped to avoid a
/// long parameter list (previously 13 arguments).
pub(crate) struct DepsCliArgs {
    pub effective_paths: Vec<PathBuf>,
    pub flags: DepsFlags,
    pub requirements: Option<String>,
    pub ignore_unused: Vec<String>,
    pub ignore_missing: Vec<String>,
    pub exclude: Vec<String>,
    pub output_file: Option<String>,
    pub cli_exclude_folders: Vec<String>,
    pub impact_package: Option<String>,
    pub venv: Option<String>,
    pub lockfile: Option<String>,
}

pub(crate) fn handle_deps<W: std::io::Write>(
    args: DepsCliArgs,
    config: &Config,
    writer: &mut W,
) -> Result<i32> {
    let mut all_exclude = args.cli_exclude_folders;
    all_exclude.extend(args.exclude);

    let mut final_ignore_unused = args.ignore_unused;
    if let Some(conf_ignored) = &config.cytoscnpy.deps.ignore_unused {
        final_ignore_unused.extend(conf_ignored.clone());
    }

    let mut final_ignore_missing = args.ignore_missing;
    if let Some(conf_ignored) = &config.cytoscnpy.deps.ignore_missing {
        final_ignore_missing.extend(conf_ignored.clone());
    }

    let venv_path = args.venv.map(PathBuf::from);
    let lockfile_path = args.lockfile.map(PathBuf::from);

    let options = crate::deps::DepsOptions {
        roots: &args.effective_paths,
        exclude: &all_exclude,
        requirements: args.requirements,
        ignore_unused: &final_ignore_unused,
        ignore_missing: &final_ignore_missing,
        verbose: args.flags.verbose,
        json: args.flags.json,
        package_mapping: config.cytoscnpy.deps.package_mapping.as_ref(),
        venv_path,
        lockfile_path,
        show_extra: args.flags.show_extra,
        show_orphans: args.flags.show_orphans,
        impact_package: args.impact_package,
    };

    if let Some(out_path) = args.output_file {
        let mut out_file = File::create(out_path)?;
        crate::commands::run_deps(&options, &mut out_file)?;
    } else {
        crate::commands::run_deps(&options, writer)?;
    }

    Ok(0)
}
