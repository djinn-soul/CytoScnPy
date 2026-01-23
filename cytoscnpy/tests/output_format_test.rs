//! Tests for various output formats.

use anyhow::{Context, Result};
use std::io::Write;

// Helper to run analysis and return output
fn run_analysis(format: &str) -> Result<String> {
    let mut file = tempfile::Builder::new().suffix(".py").tempfile()?;
    // Write sample code with issues
    writeln!(file, "import os\ndef unused_func():\n    pass\n")?;
    let path = file
        .path()
        .to_str()
        .context("Failed to get file path")?
        .to_owned();

    let mut output = Vec::new();
    let args = vec![
        path,
        "--format".to_owned(),
        format.to_owned(),
        // "--no-dead".to_owned(), // Removed to ensure findings
    ];

    // Note: run_with_args_to captures stdout.
    cytoscnpy::entry_point::run_with_args_to(args, &mut output)?;

    Ok(String::from_utf8(output)?)
}

#[test]
fn test_junit_output() -> Result<()> {
    let output = run_analysis("junit")?;
    assert!(output.contains("<testsuites>"));
    assert!(output.contains("<testcase"));
    Ok(())
}

#[test]
fn test_github_output() -> Result<()> {
    let output = run_analysis("github")?;
    // Should check for valid GitHub command format, e.g. ::error or ::warning
    // Since we provided code with unused func -> might be a warning/low severity finding or similar
    // The sample code has "import os" which might trigger something if configured,
    // and "unused_func" which triggers unused code detection.
    assert!(output.contains("::") || output.is_empty());
    // It should definitely contain something if analysis found issues.
    // unused_func should be found.
    assert!(output.contains("file="));
    assert!(output.contains("col="));

    // Strict validation against GitHub Actions command syntax
    // Pattern: ::(error|warning) file=...,line=...,col=...,title=...::message
    let re = regex::Regex::new(
        r"^::(error|warning|notice|debug) file=.+,line=\d+,col=\d+,title=[^:]+::.+$",
    )?;

    // Check that at least one line matches the full format
    let has_valid_command = output.lines().any(|line| re.is_match(line));
    assert!(
        has_valid_command,
        "Output should contain at least one valid GitHub Actions command: {output}"
    );

    // Verify paths use forward slashes (no backslashes allowed in file path part)
    let has_backslash = output
        .lines()
        .any(|line| line.contains("file=") && line.contains('\\'));
    assert!(
        !has_backslash,
        "GitHub output should use forward slashes for paths: {output}"
    );

    Ok(())
}

#[test]
fn test_sarif_output() -> Result<()> {
    let output = run_analysis("sarif")?;
    assert!(output.contains("\"version\": \"2.1.0\""));
    assert!(output.contains("\"driver\":"));
    assert!(output.contains("\"results\":"));
    Ok(())
}

#[test]
fn test_markdown_output() -> Result<()> {
    let output = run_analysis("markdown")?;
    assert!(output.contains("# CytoScnPy Analysis Report"));
    assert!(output.contains("| Category | Count |"));
    Ok(())
}

#[test]
fn test_gitlab_output() -> Result<()> {
    let output = run_analysis("gitlab")?;
    // GitLab output is a JSON array
    assert!(output.starts_with('['));
    assert!(output.contains("\"description\":"));
    assert!(output.contains("\"fingerprint\":"));
    assert!(output.contains("\"check_name\":"));
    assert!(output.contains("\"location\":"));
    Ok(())
}
