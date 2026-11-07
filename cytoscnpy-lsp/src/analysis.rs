//! Runs the CytoScnPy CLI analysis in-process on one saved file or a project.

use std::path::Path;

use crate::report::Report;

/// A report on the file at `path` as saved, with the text it was made from.
pub(crate) struct Analysis {
    pub text: String,
    pub report: Report,
}

/// Analyzes the saved file at `path` the way `cytoscnpy <path> --json
/// --force-exclude <extra_args>` does: the project configuration found from
/// the file decides which checks run, and a file the project excludes, or a
/// test file, yields an empty report.
///
/// # Errors
///
/// Returns a message when the file cannot be read or the analysis fails,
/// for example because `extra_args` holds a flag the CLI rejects.
pub(crate) fn analyze(path: &Path, extra_args: &[String]) -> Result<Analysis, String> {
    // Read the text first so fix offsets match what was analyzed, even if
    // the file changes on disk while the analysis runs.
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
    let report = run(path, &["--force-exclude"], extra_args)?;
    Ok(Analysis { text, report })
}

/// Analyzes the project in `root` the way `cytoscnpy <root> --json
/// <extra_args>` does, so a definition used only from another file is not
/// reported as unused.
///
/// # Errors
///
/// Returns a message when the analysis fails.
pub(crate) fn analyze_project(root: &Path, extra_args: &[String]) -> Result<Report, String> {
    run(root, &[], extra_args)
}

fn run(path: &Path, flags: &[&str], extra_args: &[String]) -> Result<Report, String> {
    let mut args = vec![path.to_string_lossy().into_owned(), "--json".to_owned()];
    args.extend(flags.iter().map(|&flag| flag.to_owned()));
    args.extend(extra_args.iter().cloned());
    let mut output = Vec::new();
    cytoscnpy::entry_point::run_with_args_to(args, &mut output)
        .map_err(|error| format!("Analysis of {} failed: {error:#}", path.display()))?;
    if output.iter().all(u8::is_ascii_whitespace) {
        // The CLI reports bad arguments on stderr and writes no report.
        return Err(format!(
            "Analysis of {} produced no report; check the `args` setting",
            path.display()
        ));
    }
    serde_json::from_slice(&output)
        .map_err(|error| format!("Cannot read the report for {}: {error}", path.display()))
}

/// The projects in the workspace folder `folder`, as VS Code finds them with
/// `cytoscnpy projects <folder> --json`: the folder itself plus each nested
/// folder whose own `pyproject.toml` (with a `[tool.cytoscnpy]` table) or
/// `.cytoscnpy.toml` applies when it is analyzed on its own. Falls back to
/// the folder alone.
pub(crate) fn project_roots(folder: &Path) -> Vec<std::path::PathBuf> {
    let args = vec![
        "projects".to_owned(),
        folder.to_string_lossy().into_owned(),
        "--json".to_owned(),
    ];
    let mut output = Vec::new();
    let roots: Vec<std::path::PathBuf> =
        cytoscnpy::entry_point::run_with_args_to(args, &mut output)
            .ok()
            .and_then(|_| serde_json::from_slice::<serde_json::Value>(&output).ok())
            .and_then(|value| {
                value.get("projects")?.as_array().map(|roots| {
                    roots
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(|root| folder.join(root))
                        .filter_map(|root| root.canonicalize().ok())
                        .collect()
                })
            })
            .unwrap_or_default();
    if roots.is_empty() {
        vec![folder.to_path_buf()]
    } else {
        roots
    }
}

/// The text `cytoscnpy <metric> <file>` prints, for the `cc`, `hal`, `mi`
/// and `raw` metric commands.
///
/// # Errors
///
/// Returns a message when the command fails.
pub(crate) fn metric_report(metric: &str, file: &Path) -> Result<String, String> {
    let args = vec![metric.to_owned(), file.to_string_lossy().into_owned()];
    let mut output = Vec::new();
    cytoscnpy::entry_point::run_with_args_to(args, &mut output).map_err(|error| {
        format!(
            "`cytoscnpy {metric}` failed on {}: {error:#}",
            file.display()
        )
    })?;
    Ok(String::from_utf8_lossy(&output).into_owned())
}
