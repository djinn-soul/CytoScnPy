//! Tests for `entry_point`.rs CLI argument handling and `run_with_args` function.
#![allow(clippy::unwrap_used)]

use cytoscnpy::entry_point::{run_with_args, run_with_args_to};
use std::fs;
use tempfile::{tempdir, TempDir};

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-cli-tmp");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("cli_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

/// Helper function to run CLI with output captured to suppress test noise.
fn run_with_captured_output(args: Vec<String>) -> anyhow::Result<i32> {
    let mut buffer = Vec::new();
    run_with_args_to(args, &mut buffer)
}

/// Test that --version flag works correctly.
#[test]
fn test_version_flag() {
    let result = run_with_args(vec!["--version".to_owned()]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test that --help flag works correctly.
#[test]
fn test_help_flag() {
    let result = run_with_args(vec!["--help".to_owned()]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test analyzing a single Python file.
#[test]
fn test_analyze_single_file() {
    let dir = project_tempdir();
    let file_path = dir.path().join("test_file.py");
    fs::write(&file_path, "def unused_func():\n    pass\n").unwrap();

    let result = run_with_captured_output(vec![
        "--json".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
}

/// Test analyzing with --secrets flag.
#[test]
fn test_secrets_flag() {
    let dir = project_tempdir();
    let file_path = dir.path().join("secrets_test.py");
    fs::write(&file_path, "API_KEY = 'sk-1234567890abcdef'\n").unwrap();

    let result = run_with_captured_output(vec![
        "--json".to_owned(),
        "--secrets".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
}

/// Test analyzing with --danger flag.
#[test]
fn test_danger_flag() {
    let dir = project_tempdir();
    let file_path = dir.path().join("danger_test.py");
    fs::write(&file_path, "import os\nos.system('ls')\n").unwrap();

    let result = run_with_captured_output(vec![
        "--json".to_owned(),
        "--danger".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
}

/// Test analyzing with --quality flag.
#[test]
fn test_quality_flag() {
    let dir = project_tempdir();
    let file_path = dir.path().join("quality_test.py");
    fs::write(
        &file_path,
        "def complex_func():\n    if True:\n        if True:\n            pass\n",
    )
    .unwrap();

    let result = run_with_captured_output(vec![
        "--json".to_owned(),
        "--quality".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
}

/// Test error handling for non-existent path.
#[test]
fn test_nonexistent_path() {
    let result = run_with_captured_output(vec![
        "--json".to_owned(),
        "/nonexistent/path/to/file.py".to_owned(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1); // Should return error code 1
}

/// Test the `raw` subcommand.
#[test]
fn test_raw_subcommand() {
    let dir = project_tempdir();
    let file_path = dir.path().join("raw_test.py");
    fs::write(&file_path, "x = 1\ny = 2\n").unwrap();

    let result = run_with_captured_output(vec![
        "raw".to_owned(),
        "--json".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test the `cc` (cyclomatic complexity) subcommand.
#[test]
fn test_cc_subcommand() {
    let dir = project_tempdir();
    let file_path = dir.path().join("cc_test.py");
    fs::write(&file_path, "def foo():\n    if True:\n        pass\n").unwrap();

    let result = run_with_captured_output(vec![
        "cc".to_owned(),
        "--json".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test the `hal` (Halstead metrics) subcommand.
#[test]
fn test_hal_subcommand() {
    let dir = project_tempdir();
    let file_path = dir.path().join("hal_test.py");
    fs::write(&file_path, "x = 1 + 2 * 3\n").unwrap();

    let result = run_with_captured_output(vec![
        "hal".to_owned(),
        "--json".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test the `mi` (Maintainability Index) subcommand.
#[test]
fn test_mi_subcommand() {
    let dir = project_tempdir();
    let file_path = dir.path().join("mi_test.py");
    fs::write(&file_path, "def foo():\n    pass\n").unwrap();

    let result = run_with_captured_output(vec![
        "mi".to_owned(),
        "--json".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test the `stats` subcommand.
#[test]
fn test_stats_subcommand() {
    let dir = project_tempdir();
    let file_path = dir.path().join("stats_test.py");
    fs::write(&file_path, "def foo():\n    pass\n").unwrap();

    let result = run_with_captured_output(vec![
        "stats".to_owned(),
        "--json".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test the `files` subcommand.
#[test]
fn test_files_subcommand() {
    let dir = project_tempdir();
    let file_path = dir.path().join("files_test.py");
    fs::write(&file_path, "x = 1\n").unwrap();

    let result = run_with_captured_output(vec![
        "files".to_owned(),
        "--json".to_owned(),
        dir.path().to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test --verbose flag.
#[test]
fn test_verbose_flag() {
    let dir = project_tempdir();
    let file_path = dir.path().join("verbose_test.py");
    fs::write(&file_path, "def foo():\n    pass\n").unwrap();

    let result = run_with_captured_output(vec![
        "--verbose".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
}

/// Test --confidence flag.
#[test]
fn test_confidence_flag() {
    let dir = project_tempdir();
    let file_path = dir.path().join("confidence_test.py");
    fs::write(&file_path, "def maybe_used():\n    pass\n").unwrap();

    let result = run_with_captured_output(vec![
        "--json".to_owned(),
        "--confidence".to_owned(),
        "80".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
}

/// Test --exclude-folders flag.
#[test]
fn test_exclude_folders_flag() {
    let dir = project_tempdir();
    let file_path = dir.path().join("exclude_test.py");
    fs::write(&file_path, "def foo():\n    pass\n").unwrap();

    let result = run_with_captured_output(vec![
        "--json".to_owned(),
        "--exclude-folders".to_owned(),
        "tests".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
}

/// Test --fail-on-quality flag exits with code 1 when quality issues are found.
#[test]
fn test_fail_on_quality_flag() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("quality_fail_test.py");
    // Create a deeply nested function that triggers quality issues (max nesting exceeded)
    fs::write(
        &file_path,
        "def foo():\n    if True:\n        if True:\n            if True:\n                if True:\n                    pass\n",
    )
    .unwrap();

    // With --fail-on-quality and --quality, should exit 1 due to nesting violation
    let result = run_with_captured_output(vec![
        "--quality".to_owned(),
        "--fail-on-quality".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1); // Should fail due to quality issues
}

#[test]
fn test_fail_on_quality_enables_quality_scan() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("quality_fail_test.py");
    fs::write(
        &file_path,
        "def foo():\n    if True:\n        if True:\n            if True:\n                if True:\n                    pass\n",
    )
    .unwrap();

    let result = run_with_captured_output(vec![
        "--fail-on-quality".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}

/// Test --fail-on-quality with no quality issues returns exit code 0.
#[test]
fn test_fail_on_quality_no_issues() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("quality_pass_test.py");
    // Simple function with no quality issues
    fs::write(&file_path, "def foo():\n    pass\n").unwrap();

    // With --fail-on-quality and --quality, should exit 0 (no issues)
    let result = run_with_captured_output(vec![
        "--quality".to_owned(),
        "--fail-on-quality".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0); // Should pass - no quality issues
}

/// Test --fail-on-danger enables security scanning and exits with code 1 when findings are found.
#[test]
fn test_fail_on_danger_flag() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("danger_fail_test.py");
    fs::write(&file_path, "import os\nos.system(user_input)\n").unwrap();

    let result = run_with_captured_output(vec![
        "--fail-on-danger".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}

/// Test --fail-on-secrets enables secrets scanning and exits with code 1 when findings are found.
#[test]
fn test_fail_on_secrets_flag() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("secret_config.py");
    fs::write(
        &file_path,
        "STRIPE_KEY = 'sk_live_abcdefghijklmnopqrstuvwx'\n",
    )
    .unwrap();

    let result = run_with_captured_output(vec![
        "--fail-on-secrets".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}

/// Test dependency gates enable dependency analysis on the main analysis command.
#[test]
fn test_fail_on_dependency_flags_in_main_analysis() {
    let dir = tempdir().unwrap();
    let pyproject_path = dir.path().join("pyproject.toml");
    let file_path = dir.path().join("main.py");

    fs::write(
        &pyproject_path,
        r#"
[project]
name = "dep-check"
version = "0.1.0"
dependencies = ["requests", "unused-dep"]
"#,
    )
    .unwrap();
    fs::write(&file_path, "import requests\nimport missing_dep\n").unwrap();

    let missing_result = run_with_captured_output(vec![
        "--fail-on-missing-deps".to_owned(),
        "--json".to_owned(),
        dir.path().to_string_lossy().to_string(),
    ]);
    assert!(missing_result.is_ok());
    assert_eq!(missing_result.unwrap(), 1);

    let unused_result = run_with_captured_output(vec![
        "--fail-on-unused-deps".to_owned(),
        "--json".to_owned(),
        dir.path().to_string_lossy().to_string(),
    ]);
    assert!(unused_result.is_ok());
    assert_eq!(unused_result.unwrap(), 1);
}

/// Test --fail-on-any enables all main analysis gates.
#[test]
fn test_fail_on_any_flag_in_main_analysis() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("secret_config.py"),
        "STRIPE_KEY = 'sk_live_abcdefghijklmnopqrstuvwx'\n",
    )
    .unwrap();

    let result = run_with_captured_output(vec![
        "--fail-on-any".to_owned(),
        "--json".to_owned(),
        dir.path().to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}

#[test]
fn test_fail_on_any_file_target_uses_parent_for_dependency_declarations() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.py");
    fs::write(
        dir.path().join("pyproject.toml"),
        r#"
[project]
name = "dep-check"
version = "0.1.0"
dependencies = ["requests"]
"#,
    )
    .unwrap();
    fs::write(&file_path, "import requests\nprint(requests.__version__)\n").unwrap();

    let result = run_with_captured_output(vec![
        "--fail-on-any".to_owned(),
        "--json".to_owned(),
        file_path.to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Explicit --fail-threshold should override --fail-on-any's zero-tolerance default.
#[test]
fn test_fail_on_any_respects_explicit_fail_threshold() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("unused.py"),
        "def unused_function():\n    pass\n",
    )
    .unwrap();

    let result = run_with_captured_output(vec![
        "--fail-on-any".to_owned(),
        "--fail-threshold".to_owned(),
        "1000".to_owned(),
        "--json".to_owned(),
        dir.path().to_string_lossy().to_string(),
    ]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test config fail gates also enable their required analyses.
#[test]
fn test_config_fail_gates_enable_required_scans() {
    let security_dir = tempdir().unwrap();
    fs::write(
        security_dir.path().join(".cytoscnpy.toml"),
        "[cytoscnpy]\nfail_on_danger = true\nfail_on_secrets = true\n",
    )
    .unwrap();
    fs::write(
        security_dir.path().join("security.py"),
        "import os\nos.system(user_input)\nSTRIPE_KEY = 'sk_live_abcdefghijklmnopqrstuvwx'\n",
    )
    .unwrap();

    let security_result = run_with_captured_output(vec![
        "--json".to_owned(),
        security_dir.path().to_string_lossy().to_string(),
    ]);
    assert!(security_result.is_ok());
    assert_eq!(security_result.unwrap(), 1);

    let deps_dir = tempdir().unwrap();
    fs::write(
        deps_dir.path().join(".cytoscnpy.toml"),
        "[cytoscnpy.deps]\nfail_on_missing = true\n",
    )
    .unwrap();
    fs::write(
        deps_dir.path().join("pyproject.toml"),
        r#"
[project]
name = "dep-check"
version = "0.1.0"
dependencies = ["requests"]
"#,
    )
    .unwrap();
    fs::write(deps_dir.path().join("main.py"), "import missing_dep\n").unwrap();

    let deps_result = run_with_captured_output(vec![
        "--json".to_owned(),
        deps_dir.path().to_string_lossy().to_string(),
    ]);
    assert!(deps_result.is_ok());
    assert_eq!(deps_result.unwrap(), 1);
}

/// VS Code integration should honor project config fail gates from pyproject.toml
/// and .cytoscnpy.toml when editor settings are unset.
#[test]
fn test_vscode_client_honors_config_fail_gates_for_scan_selection() {
    let security_dir = tempdir().unwrap();
    fs::write(
        security_dir.path().join("pyproject.toml"),
        "[tool.cytoscnpy]\nfail_on_danger = true\nfail_on_secrets = true\n",
    )
    .unwrap();
    fs::write(
        security_dir.path().join("security.py"),
        "import os\nos.system(user_input)\nSTRIPE_KEY = 'sk_live_abcdefghijklmnopqrstuvwx'\n",
    )
    .unwrap();

    let security_result = run_with_captured_output(vec![
        "--client".to_owned(),
        "vscode".to_owned(),
        "--json".to_owned(),
        security_dir.path().to_string_lossy().to_string(),
    ]);
    assert!(security_result.is_ok());
    assert_eq!(security_result.unwrap(), 1);

    let deps_dir = tempdir().unwrap();
    fs::write(
        deps_dir.path().join(".cytoscnpy.toml"),
        "[cytoscnpy.deps]\nfail_on_missing = true\n",
    )
    .unwrap();
    fs::write(
        deps_dir.path().join("pyproject.toml"),
        r#"
[project]
name = "dep-check"
version = "0.1.0"
dependencies = ["requests"]
"#,
    )
    .unwrap();
    fs::write(deps_dir.path().join("main.py"), "import missing_dep\n").unwrap();

    let deps_result = run_with_captured_output(vec![
        "--client".to_owned(),
        "vscode".to_owned(),
        "--json".to_owned(),
        deps_dir.path().to_string_lossy().to_string(),
    ]);
    assert!(deps_result.is_ok());
    assert_eq!(deps_result.unwrap(), 1);
}

/// VS Code integration still honors explicit CLI fail gates.
#[test]
fn test_vscode_client_honors_cli_fail_gates() {
    let security_dir = tempdir().unwrap();
    fs::write(
        security_dir.path().join("security.py"),
        "import os\nos.system(user_input)\n",
    )
    .unwrap();

    let security_result = run_with_captured_output(vec![
        "--client".to_owned(),
        "vscode".to_owned(),
        "--fail-on-danger".to_owned(),
        "--json".to_owned(),
        security_dir.path().to_string_lossy().to_string(),
    ]);
    assert!(security_result.is_ok());
    assert_eq!(security_result.unwrap(), 1);

    let deps_dir = tempdir().unwrap();
    fs::write(
        deps_dir.path().join("pyproject.toml"),
        r#"
[project]
name = "dep-check"
version = "0.1.0"
dependencies = ["requests"]
"#,
    )
    .unwrap();
    fs::write(deps_dir.path().join("main.py"), "import missing_dep\n").unwrap();

    let deps_result = run_with_captured_output(vec![
        "--client".to_owned(),
        "vscode".to_owned(),
        "--fail-on-missing-deps".to_owned(),
        "--json".to_owned(),
        deps_dir.path().to_string_lossy().to_string(),
    ]);
    assert!(deps_result.is_ok());
    assert_eq!(deps_result.unwrap(), 1);
}

#[test]
fn test_main_analysis_fail_on_any_includes_transitive_deps() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("pyproject.toml"),
        r#"
[project]
name = "dep-check"
version = "0.1.0"
dependencies = ["requests"]
"#,
    )
    .unwrap();
    fs::write(dir.path().join("main.py"), "import urllib3\n").unwrap();
    fs::write(
        dir.path().join("uv.lock"),
        r#"
version = 1

[[package]]
name = "requests"
version = "2.31.0"
dependencies = [
  { name = "urllib3" },
]

[[package]]
name = "urllib3"
version = "2.0.0"
"#,
    )
    .unwrap();

    let result = run_with_captured_output(vec![
        "--deps".to_owned(),
        "--fail-on-any".to_owned(),
        "--json".to_owned(),
        dir.path().to_string_lossy().to_string(),
    ]);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}
