use super::*;

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
