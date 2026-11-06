use crate::cli::{Cli, Commands};
use anyhow::Result;

/// Resolves subcommand paths, defaulting to `.` if empty, and checks existence.
/// Returns `Ok(Vec<PathBuf>)` if all paths exist, `Err(1)` if any doesn't.
pub(crate) fn resolve_subcommand_paths(
    paths: Vec<std::path::PathBuf>,
    root: Option<std::path::PathBuf>,
) -> Result<Vec<std::path::PathBuf>, i32> {
    // If --root is provided, it's the only path we care about
    let final_paths = if let Some(r) = root {
        vec![r]
    } else if paths.is_empty() {
        vec![std::path::PathBuf::from(".")]
    } else {
        paths
    };

    for path in &final_paths {
        if !path.exists() {
            eprintln!(
                "Error: The file or directory '{}' does not exist.",
                path.display()
            );
            return Err(1);
        }
    }
    Ok(final_paths)
}

/// Validates and prepares an output file path for a subcommand.
/// Returns the validated path string, or propagates errors.
pub(crate) fn prepare_output_path(
    output_file: Option<String>,
    analysis_root: &std::path::Path,
) -> Result<Option<String>> {
    match output_file {
        Some(out) => Ok(Some(
            crate::utils::validate_output_path(std::path::Path::new(&out), Some(analysis_root))?
                .to_string_lossy()
                .to_string(),
        )),
        None => Ok(None),
    }
}

/// Merges subcommand-specific excludes with global excludes from config.
pub(crate) fn merge_excludes(
    subcommand_excludes: Vec<String>,
    global_excludes: &[String],
) -> Vec<String> {
    let mut merged = subcommand_excludes;
    merged.extend(global_excludes.iter().cloned());
    merged
}

/// Validates that --root and positional paths are not used together.
/// Returns Ok(()) if valid, Err(1) if both are provided.
pub(crate) fn validate_path_args(args: &crate::cli::PathArgs) -> Result<(), i32> {
    if args.root.is_some() && !args.paths.is_empty() {
        eprintln!("Error: Cannot use both --root and positional path arguments");
        return Err(1);
    }
    Ok(())
}

/// Collects all target paths from global and subcommand arguments.
/// The path arguments of the subcommand being run, if it takes any.
fn subcommand_path_args(cli: &Cli) -> Option<&crate::cli::PathArgs> {
    match cli.command.as_ref()? {
        Commands::Raw { common, .. }
        | Commands::Cc { common, .. }
        | Commands::Hal { common, .. }
        | Commands::Mi { common, .. } => Some(&common.paths),
        Commands::Projects { args } => Some(&args.paths),
        Commands::Files { args, .. } => Some(&args.paths),
        Commands::Stats { paths, .. }
        | Commands::Deslop {
            args: crate::cli::DeslopArgs { paths, .. },
        }
        | Commands::Deps {
            args: crate::cli::DepsArgs { paths, .. },
        } => Some(paths),
        Commands::McpServer | Commands::Init => None,
    }
}

pub(crate) fn collect_all_target_paths(cli: &Cli) -> Vec<std::path::PathBuf> {
    let mut all_target_paths = cli.paths.paths.clone();
    if let Some(paths) = subcommand_path_args(cli) {
        if let Some(r) = &paths.root {
            all_target_paths.push(r.clone());
        } else {
            all_target_paths.extend(paths.paths.iter().cloned());
        }
    }
    all_target_paths
}

/// Resolves effective paths and analysis root based on CLI arguments.
pub(crate) fn resolve_analysis_context(
    cli: &Cli,
    all_target_paths: &[std::path::PathBuf],
) -> (Vec<std::path::PathBuf>, std::path::PathBuf) {
    // --root, global or on the subcommand, is the analysis path AND the
    // containment boundary for everything the command writes.
    let root = cli
        .paths
        .root
        .as_ref()
        .or_else(|| subcommand_path_args(cli).and_then(|paths| paths.root.as_ref()));
    if let Some(root) = root {
        return (vec![root.clone()], root.clone());
    }

    let mut root = std::path::PathBuf::from(".");
    if let Some(first_abs) = all_target_paths.iter().find(|p| p.is_absolute()) {
        // Determine common ancestor for absolute paths
        let mut common = if first_abs.is_dir() {
            first_abs.clone()
        } else {
            first_abs
                .parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or_else(|| first_abs.clone())
        };

        for path in all_target_paths.iter().filter(|p| p.is_absolute()) {
            while !path.starts_with(&common) {
                if let Some(parent) = common.parent() {
                    common = parent.to_path_buf();
                } else {
                    break;
                }
            }
        }
        root = common;
    }

    let paths = if cli.paths.paths.is_empty() {
        // If it's a subcommand call, we might not have global paths.
        // But loading config from the first subcommand path is better than ".".
        if let Some(first) = all_target_paths.first() {
            vec![first.clone()]
        } else {
            vec![std::path::PathBuf::from(".")]
        }
    } else {
        cli.paths.paths.clone()
    };

    (paths, root)
}

#[cfg(test)]
mod tests {
    use super::{collect_all_target_paths, resolve_analysis_context};
    use crate::cli::Cli;
    use clap::Parser;
    use std::path::PathBuf;

    fn analysis_root(args: &[&str]) -> PathBuf {
        let cli = Cli::try_parse_from(args).unwrap();
        let targets = collect_all_target_paths(&cli);
        resolve_analysis_context(&cli, &targets).1
    }

    #[test]
    fn subcommand_root_bounds_output_like_global_root() {
        assert_eq!(
            analysis_root(&["cytoscnpy", "--root", "proj"]),
            PathBuf::from("proj")
        );
        assert_eq!(
            analysis_root(&["cytoscnpy", "stats", "--root", "proj"]),
            PathBuf::from("proj")
        );
        assert_eq!(
            analysis_root(&["cytoscnpy", "deslop", "--root", "proj"]),
            PathBuf::from("proj")
        );
    }

    #[test]
    fn relative_paths_keep_working_directory_as_root() {
        assert_eq!(
            analysis_root(&["cytoscnpy", "stats", "proj"]),
            PathBuf::from(".")
        );
    }
}
