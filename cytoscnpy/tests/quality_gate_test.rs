//! Integration tests for CI/CD quality gate feature (--fail-under flag)
//!
//! NOTE: These tests require the binary to be built first (`cargo build`).
//! They are marked #[ignore] because CI coverage runs use a different target directory.
//! Run locally with: `cargo test --test quality_gate_test -- --ignored`

use std::fs;
use std::process::Command;
use tempfile::tempdir;

/// Helper to run cytoscnpy and capture output
fn run_cytoscnpy(args: &[&str], dir: &std::path::Path) -> std::process::Output {
    // Get the path to the binary - CARGO_MANIFEST_DIR points to cytoscnpy/
    // so we go up to workspace root for target/
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap();

    #[cfg(windows)]
    let binary_name = "cytoscnpy-bin.exe";
    #[cfg(not(windows))]
    let binary_name = "cytoscnpy-bin";

    let binary_path = workspace_root.join("target/debug").join(binary_name);

    Command::new(&binary_path)
        .args(args)
        .current_dir(dir)
        .output()
        .expect(&format!("Failed to execute cytoscnpy at {:?}", binary_path))
}

#[test]
#[ignore] // Requires pre-built binary
fn test_fail_under_passes_when_below_threshold() {
    let temp_dir = tempdir().unwrap();

    // Create a clean Python file with no unused code
    let file_path = temp_dir.path().join("clean.py");
    fs::write(
        &file_path,
        r#"
def used_function():
    return 42

result = used_function()
print(result)
"#,
    )
    .unwrap();

    let output = run_cytoscnpy(&[".", "--fail-under", "50", "--json"], temp_dir.path());

    // Should pass (exit code 0) because there's minimal unused code
    assert!(
        output.status.success(),
        "Expected success but got failure. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore] // Requires pre-built binary
fn test_fail_under_fails_when_above_threshold() {
    let temp_dir = tempdir().unwrap();

    // Create Python files with lots of unused code (high percentage)
    for i in 0..3 {
        let file_path = temp_dir.path().join(format!("unused_{}.py", i));
        fs::write(
            &file_path,
            r#"
def unused_function_1():
    pass

def unused_function_2():
    pass

def unused_function_3():
    pass

class UnusedClass:
    pass
"#,
        )
        .unwrap();
    }

    // Very low threshold - should fail
    let output = run_cytoscnpy(&[".", "--fail-under", "0.1", "--json"], temp_dir.path());

    // Should fail (exit code 1) because percentage exceeds ultra-low threshold
    assert!(
        !output.status.success(),
        "Expected failure but got success. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that the error message is present
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Quality gate FAILED"),
        "Expected 'Quality gate FAILED' message. Got: {}",
        stderr
    );
}

#[test]
#[ignore] // Requires pre-built binary
fn test_fail_under_with_env_var() {
    let temp_dir = tempdir().unwrap();

    // Create a file with some unused code
    let file_path = temp_dir.path().join("mixed.py");
    fs::write(
        &file_path,
        r#"
def used_function():
    return 42

def unused_function():
    pass

result = used_function()
"#,
    )
    .unwrap();

    // Use helper function's path logic
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap();
    #[cfg(windows)]
    let binary_name = "cytoscnpy-bin.exe";
    #[cfg(not(windows))]
    let binary_name = "cytoscnpy-bin";
    let binary_path = workspace_root.join("target/debug").join(binary_name);

    // Run with env var set to ultra-low threshold
    let output = Command::new(&binary_path)
        .args(&[".", "--json"])
        .current_dir(temp_dir.path())
        .env("CYTOSCNPY_FAIL_THRESHOLD", "0.01")
        .output()
        .expect("Failed to execute cytoscnpy");

    // Should fail due to env var threshold
    assert!(
        !output.status.success(),
        "Expected failure from env var threshold. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore] // Requires pre-built binary
fn test_fail_under_cli_overrides_env_var() {
    let temp_dir = tempdir().unwrap();

    // Create a file with some unused code
    let file_path = temp_dir.path().join("test.py");
    fs::write(
        &file_path,
        r#"
def unused_function():
    pass
"#,
    )
    .unwrap();

    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap();
    #[cfg(windows)]
    let binary_name = "cytoscnpy-bin.exe";
    #[cfg(not(windows))]
    let binary_name = "cytoscnpy-bin";
    let binary_path = workspace_root.join("target/debug").join(binary_name);

    // Env var says fail at 0.01%, but CLI says 1000% (should always pass)
    let output = Command::new(&binary_path)
        .args(&[".", "--fail-under", "1000", "--json"])
        .current_dir(temp_dir.path())
        .env("CYTOSCNPY_FAIL_THRESHOLD", "0.01")
        .output()
        .expect("Failed to execute cytoscnpy");

    // Should pass because CLI overrides env var
    assert!(
        output.status.success(),
        "Expected CLI to override env var. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore] // Requires pre-built binary
fn test_no_quality_gate_when_not_specified() {
    let temp_dir = tempdir().unwrap();

    // Create a file with tons of unused code
    let file_path = temp_dir.path().join("lots_unused.py");
    fs::write(
        &file_path,
        r#"
def unused1(): pass
def unused2(): pass
def unused3(): pass
def unused4(): pass
def unused5(): pass
class Unused1: pass
class Unused2: pass
"#,
    )
    .unwrap();

    // Run without --fail-under and without env var
    let output = run_cytoscnpy(&[".", "--json"], temp_dir.path());

    // Should always pass when quality gate is not enabled
    assert!(
        output.status.success(),
        "Expected success when --fail-under not specified. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
