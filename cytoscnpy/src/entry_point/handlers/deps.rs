use crate::config::Config;
use anyhow::Result;
use std::fs::File;
use std::path::PathBuf;

pub(crate) fn handle_deps<W: std::io::Write>(
    effective_paths: &[PathBuf],
    json: bool,
    requirements: Option<String>,
    ignore_unused: Vec<String>,
    ignore_missing: Vec<String>,
    exclude: Vec<String>,
    output_file: Option<String>,
    config: &Config,
    cli_exclude_folders: &[String],
    verbose: bool,
    writer: &mut W,
) -> Result<i32> {
    let mut all_exclude = cli_exclude_folders.to_vec();
    all_exclude.extend(exclude);

    let mut final_ignore_unused = ignore_unused;
    if let Some(conf_ignored) = &config.cytoscnpy.deps.ignore_unused {
        final_ignore_unused.extend(conf_ignored.clone());
    }

    let mut final_ignore_missing = ignore_missing;
    if let Some(conf_ignored) = &config.cytoscnpy.deps.ignore_missing {
        final_ignore_missing.extend(conf_ignored.clone());
    }

    let options = crate::deps::DepsOptions {
        roots: effective_paths,
        exclude: &all_exclude,
        requirements,
        ignore_unused: &final_ignore_unused,
        ignore_missing: &final_ignore_missing,
        verbose,
        json,
    };

    if let Some(out_path) = output_file {
        let mut out_file = File::create(out_path)?;
        crate::commands::run_deps(&options, &mut out_file)?;
    } else {
        crate::commands::run_deps(&options, writer)?;
    }

    Ok(0)
}
