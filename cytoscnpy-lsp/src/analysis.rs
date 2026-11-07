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
            "Analysis of {} produced no report; check the `args` initialization option",
            path.display()
        ));
    }
    serde_json::from_slice(&output)
        .map_err(|error| format!("Cannot read the report for {}: {error}", path.display()))
}
