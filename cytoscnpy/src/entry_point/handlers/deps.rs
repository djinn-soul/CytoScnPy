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
    pub fail_on_any: bool,
    pub fail_on_unused: bool,
    pub fail_on_missing: bool,
    pub fail_on_extra_installed: bool,
    pub fail_on_orphans: bool,
    pub include_dev_unused: bool,
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

    let fail_on_unused = args.flags.fail_on_any
        || args.flags.fail_on_unused
        || config.cytoscnpy.deps.fail_on_unused.unwrap_or(false);
    let fail_on_missing = args.flags.fail_on_any
        || args.flags.fail_on_missing
        || config.cytoscnpy.deps.fail_on_missing.unwrap_or(false);
    let fail_on_extra_installed = args.flags.fail_on_extra_installed
        || args.flags.fail_on_any
        || config
            .cytoscnpy
            .deps
            .fail_on_extra_installed
            .unwrap_or(false);
    let fail_on_orphans = args.flags.fail_on_any
        || args.flags.fail_on_orphans
        || config.cytoscnpy.deps.fail_on_orphans.unwrap_or(false);

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
        show_extra: args.flags.show_extra || fail_on_extra_installed,
        show_orphans: args.flags.show_orphans || fail_on_orphans,
        impact_package: args.impact_package,
        include_dev_unused: args.flags.include_dev_unused,
    };

    let result = if let Some(out_path) = args.output_file {
        let mut out_file = File::create(out_path)?;
        crate::commands::run_deps(&options, &mut out_file)?
    } else {
        crate::commands::run_deps(&options, writer)?
    };

    let should_fail = (fail_on_unused && !result.unused.is_empty())
        || (fail_on_missing && !result.missing.is_empty())
        || (fail_on_missing && !result.transitive.is_empty())
        || (fail_on_missing && !result.dev_in_production.is_empty())
        || (fail_on_unused && !result.stdlib.is_empty())
        || (fail_on_extra_installed && !result.extra_installed.is_empty())
        || (fail_on_orphans && !result.orphan_installed.is_empty());

    if should_fail && !args.flags.json {
        eprintln!("\n[GATE] Dependency analysis failed due to gated findings.");
    }

    Ok(i32::from(should_fail))
}
