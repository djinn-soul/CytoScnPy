//! Tests for the --root CLI flag functionality.
//!
//! The --root flag provides an alternative to the positional path argument.
//! It allows running CytoScnPy from any directory while specifying an
//! explicit project root for path containment security.
//!
//! Design:
//! - Either use positional path(s) OR --root, but NOT both
//! - --root serves as BOTH analysis target AND containment boundary
//! - This is useful for CI/scripts that invoke from different directories

use std::fs;
use std::path::Path;
use tempfile::tempdir;

/// Helper to run CytoScnPy and capture output
fn run_cytoscnpy(args: Vec<&str>) -> (i32, String) {
    let mut output = Vec::new();
    let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let exit_code = cytoscnpy::entry_point::run_with_args_to(args_owned, &mut output).unwrap_or(1);
    let output_str = String::from_utf8_lossy(&output).to_string();
    (exit_code, output_str)
}

/// Helper to create a test Python project
fn create_test_project(dir: &Path) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("src/main.py"),
        r"
def used_function():
    return 42

def unused_function():
    return 0

print(used_function())
",
    )
    .unwrap();
}

// ============================================================================
// TDD TESTS - These should FAIL initially and PASS after implementation
// ============================================================================

#[test]
fn test_root_flag_exists_in_help() {
    // The --root flag should appear in help output
    let (exit_code, output) = run_cytoscnpy(vec!["--help"]);

    assert_eq!(exit_code, 0, "Help should exit with 0");
    assert!(
        output.contains("--root"),
        "Help should mention --root flag. Got:\n{output}"
    );
}

#[test]
fn test_root_flag_alone_works() {
    // Using --root without positional path should work
    let temp_dir = tempdir().unwrap();
    let project_root = temp_dir.path().join("myproject");
    create_test_project(&project_root);

    let project_path = project_root.to_string_lossy().to_string();
    let (exit_code, output) = run_cytoscnpy(vec!["--root", &project_path, "--json"]);

    // Should succeed and find the files
    assert_eq!(
        exit_code, 0,
        "Should succeed with --root. Output:\n{output}"
    );
    assert!(
        output.contains("unused_function") || output.contains("main.py"),
        "Should analyze files in --root directory. Output:\n{output}"
    );
}

#[test]
fn test_root_and_path_conflict() {
    // Using both --root and positional path should ERROR
    let temp_dir = tempdir().unwrap();
    let project_root = temp_dir.path().join("myproject");
    create_test_project(&project_root);

    let project_path = project_root.to_string_lossy().to_string();

    // Try to use both positional path AND --root
    let (exit_code, _output) = run_cytoscnpy(vec!["./src", "--root", &project_path]);

    // Should fail with an error (clap sends this to stderr, not stdout)
    assert_eq!(
        exit_code, 1,
        "Should fail when both path and --root are given"
    );
    // Note: The actual error message "cannot be used with" goes to stderr,
    // which is not captured in our test helper. Exit code 1 is sufficient proof.
}

#[test]
fn test_root_flag_allows_absolute_paths_from_different_cwd() {
    // This is the KEY use case: running from a different directory
    // Using --root should allow operations on absolute paths within that root
    let temp_dir = tempdir().unwrap();
    let project_root = temp_dir.path().join("myproject");
    create_test_project(&project_root);

    // Save current dir
    let original_dir = std::env::current_dir().unwrap();

    // Change to a DIFFERENT directory (not the project)
    let other_dir = temp_dir.path().join("other");
    fs::create_dir_all(&other_dir).unwrap();
    std::env::set_current_dir(&other_dir).unwrap();

    // Now run with --root pointing to the project
    let project_path = project_root.to_string_lossy().to_string();
    let result = run_cytoscnpy(vec!["--root", &project_path, "--json"]);

    // Restore original dir before assertions
    std::env::set_current_dir(original_dir).unwrap();

    let (exit_code, output) = result;

    // Should succeed even though CWD is different
    assert_eq!(
        exit_code, 0,
        "Should succeed when running from different directory with --root. Output:\n{output}"
    );
}

#[test]
fn test_root_flag_with_stats_subcommand() {
    // --root should work with stats subcommand
    let temp_dir = tempdir().unwrap();
    let project_root = temp_dir.path().join("myproject");
    create_test_project(&project_root);

    let project_path = project_root.to_string_lossy().to_string();

    let (exit_code, output) = run_cytoscnpy(vec!["stats", "--root", &project_path, "--json"]);

    assert_eq!(exit_code, 0, "Stats with --root should succeed");
    assert!(
        output.contains("total_files") || output.contains("total_lines"),
        "Should produce stats output. Got:\n{output}"
    );
}

#[test]
fn test_default_path_behavior_unchanged() {
    // Without --root, the existing positional path behavior should work exactly as before
    let temp_dir = tempdir().unwrap();
    create_test_project(temp_dir.path());

    let project_path = temp_dir.path().to_string_lossy().to_string();
    let (exit_code, output) = run_cytoscnpy(vec![&project_path, "--json"]);

    assert_eq!(exit_code, 0, "Default path behavior should still work");
    assert!(
        output.contains("unused_function") || output.contains("main.py"),
        "Should analyze files. Output:\n{output}"
    );
}

#[test]
fn test_root_flag_help_text() {
    // The help text should explain what --root does
    let (exit_code, output) = run_cytoscnpy(vec!["--help"]);

    assert_eq!(exit_code, 0);
    // Check for meaningful help text about --root
    let has_root_description = output.contains("root")
        && (output.contains("containment")
            || output.contains("project")
            || output.contains("security")
            || output.contains("analysis"));

    assert!(
        has_root_description,
        "--root should have descriptive help text. Got:\n{output}"
    );
}
