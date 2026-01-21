use anyhow::Result;
use std::io::Write;
use tempfile::NamedTempFile;

// Helper to run analysis and return output
fn run_analysis(format: &str) -> Result<String> {
    // Create a temp file in root-level temp dir
    std::fs::create_dir_all("temp_snapshots")?;
    let mut file = tempfile::Builder::new()
        .suffix(".py")
        .prefix("snapshot_test_")
        .tempfile_in("temp_snapshots")?;

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

    // Close the file handle but keep the file on disk for analysis
    let temp_path = file.into_temp_path();
    let path = temp_path.to_str().unwrap().to_string();

    let mut output = Vec::new();
    let args = vec![
        path.clone(),
        "--format".to_string(),
        format.to_string(),
        "--quality".to_string(), // Force quality check
    ];

    // Note: run_with_args_to captures stdout.
    cytoscnpy::entry_point::run_with_args_to(args, &mut output)?;

    let output_str = String::from_utf8_lossy(&output).to_string();

    // Sanitize output to make it stable across runs/machines
    let sanitized = sanitize_output(&output_str, &path, format);

    Ok(sanitized)
}

fn sanitize_output(output: &str, file_path: &str, format: &str) -> String {
    // 0. Strip ANSI escape codes
    let ansi_re = regex::Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    let mut s = ansi_re.replace_all(output, "").to_string();

    // 1. Normalize line endings and slashes globally first
    // Handle JSON escaped backslashes (\\) first, then single backslashes
    s = s
        .replace("\r\n", "\n")
        .replace("\\\\", "/")
        .replace('\\', "/");
    let normalized_path = file_path.replace('\\', "/");

    // 2. Replace temporary file path with [FILE]
    // Use regex to capture the path and any optional ":line" suffix.
    // We want the padding to appear AFTER the suffix, so "[FILE]:5       " instead of "[FILE]       :5".
    let escaped_path = regex::escape(&normalized_path);
    // Regex matches the path optionally followed by ":<digits>"
    // The (?::\d+)? part is a non-capturing group for the colon, but capturing the digits would be fine too.
    // We just want to match the whole extent.
    let re_path_loc = regex::Regex::new(&format!(r"{}(:\d+)?", escaped_path)).unwrap();

    // We can't use replace_all blindly because we need to calculate padding based on the match length
    // But actually, the match length is strictly: path.len() + suffix.len()
    // The replacement should be: "[FILE]" + suffix + padding
    // Padding should ensure the total length equals the match length.
    // So replacement.len() == match.len()
    // "[FILE]".len() + suffix.len() + padding.len() == path.len() + suffix.len()
    // padding.len() == path.len() - "[FILE]".len()
    // This is constant!

    // So we can compute the padding once.
    let target = "[FILE]";
    if format == "text" && normalized_path.len() > target.len() {
        let diff = normalized_path.len() - target.len();
        let padding = " ".repeat(diff);
        // Replace "path(:suffix)?" with "[FILE]$1" + padding
        let replace_with = format!("{}$1{}", target, padding);
        s = re_path_loc.replace_all(&s, &replace_with).to_string();
    } else {
        // For other formats, simplest replacement is fine, but using the regex ensures consistent matching behavior
        // Actually, just simple string sub is safer/faster if we don't care about suffix
        s = s.replace(&normalized_path, target);
    }

    // 3. Sanitize timing info "Analysis completed in 0.03s" or similar
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
