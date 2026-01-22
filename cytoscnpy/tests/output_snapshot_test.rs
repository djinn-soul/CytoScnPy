//! Snapshot tests for output formatting
use anyhow::Result;
use std::io::Write;

// Helper to run analysis and return output
fn run_analysis(format: &str) -> Result<String> {
    // Create a temp file in root-level temp dir
    let temp_dir = "temp_snapshots";
    std::fs::create_dir_all(temp_dir)?;
    let mut file = tempfile::Builder::new()
        .suffix(".py")
        .prefix("snapshot_test_")
        .tempfile_in(temp_dir)?;

    // Write sample code (based on known working pattern)
    writeln!(
        file,
        r#"
import os
import sys

def unused_func():
    print("I am unused")

def main():
    unused_var = 10
    print("done")

if __name__ == "__main__":
    main()
"#
    )?;

    // Get the relative path for analysis to keep text output clean and stable
    let file_name = file
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let relative_path = format!("{}/{}", temp_dir, file_name);

    let mut output = Vec::new();
    let args = vec![
        relative_path.clone(),
        "--format".to_string(),
        format.to_string(),
        "--quality".to_string(), // Force quality check
    ];

    // Note: run_with_args_to captures stdout.
    cytoscnpy::entry_point::run_with_args_to(args, &mut output)?;

    let output_str = String::from_utf8_lossy(&output).to_string();

    // Sanitize output to make it stable across runs/machines
    let sanitized = sanitize_output(&output_str, &relative_path, format);

    Ok(sanitized)
}

fn sanitize_output(output: &str, file_path: &str, _format: &str) -> String {
    // 0. Strip ANSI escape codes
    let ansi_re = regex::Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    let mut s = ansi_re.replace_all(output, "").to_string();

    // 1. Normalize line endings and slashes globally first
    s = s
        .replace("\r\n", "\n")
        .replace("\\\\", "/")
        .replace('\\', "/");
    let normalized_path = file_path.replace('\\', "/");
    let filename = std::path::Path::new(&normalized_path)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();

    // 2. Replace temporary file path or filename with [FILE]
    // We replace the full path first, then any remaining instances of the random filename.
    s = s.replace(&normalized_path, "[FILE]");
    s = s.replace(filename, "[FILE]");

    // 3. Sanitize timing info
    let re_time = regex::Regex::new(r"(Analysis completed|Completed) in \d+\.\d+s").unwrap();
    s = re_time.replace_all(&s, "$1 in [TIME]s").to_string();

    s
}

#[test]
fn snapshot_text() {
    let output = run_analysis("text").unwrap();
    insta::assert_snapshot!("text_output", output);
}

#[test]
fn snapshot_json() {
    let output = run_analysis("json").unwrap();
    // JSON might have non-deterministic order of fields or list items if not sorted.
    // CytoScnPy implementation usually pushes to vectors, so order should be stable if traversal is stable.
    // Parallel traversal (Rayon) might make order unstable!
    // However, for a single file, rayon might not split much or at all.
    // If unstable, we'll need to deserialize and sort. Check if output is stable first.
    insta::assert_snapshot!("json_output", output);
}

#[test]
fn snapshot_junit() {
    let output = run_analysis("junit").unwrap();
    insta::assert_snapshot!("junit_output", output);
}

#[test]
fn snapshot_github() {
    let output = run_analysis("github").unwrap();
    insta::assert_snapshot!("github_output", output);
}

#[test]
fn snapshot_gitlab() {
    let output = run_analysis("gitlab").unwrap();
    insta::assert_snapshot!("gitlab_output", output);
}

#[test]
fn snapshot_markdown() {
    let output = run_analysis("markdown").unwrap();
    insta::assert_snapshot!("markdown_output", output);
}

#[test]
fn snapshot_sarif() {
    let output = run_analysis("sarif").unwrap();
    insta::assert_snapshot!("sarif_output", output);
}
