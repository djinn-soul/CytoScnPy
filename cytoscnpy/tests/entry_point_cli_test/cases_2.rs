use super::*;

/// Test --fail-on-quality with no quality issues returns exit code 0.
#[test]
fn test_fail_on_quality_no_issues() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("quality_pass.py");
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
    let file_path = dir.path().join("danger_fail.py");
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
        "100".to_owned(),
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
