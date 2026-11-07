use super::*;

#[test]
fn test_deps_literal_dynamic_imports_count_as_runtime_usage() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["requests"]
"#,
    )?;
    fs::write(
        root.join("app.py"),
        "import importlib\nplugin = importlib.import_module('requests.sessions')\nother = __import__('rich.console')\n",
    )?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(!output.contains("Unused Dependencies (CSP-R002)"));
    assert!(!output.contains("requests"));
    assert!(output.contains("Missing Dependencies (CSP-R001)"));
    assert!(output.contains("rich"));
    Ok(())
}

#[test]
fn test_deps_aliased_literal_dynamic_imports_count_as_runtime_usage() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["requests"]
"#,
    )?;
    fs::write(
        root.join("app.py"),
        "import importlib as il\nfrom importlib import import_module as im\nplugin = il.import_module('requests.sessions')\nother = im('rich.console')\n",
    )?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(!output.contains("Unused Dependencies (CSP-R002)"));
    assert!(!output.contains("requests"));
    assert!(output.contains("Missing Dependencies (CSP-R001)"));
    assert!(output.contains("rich"));
    Ok(())
}

#[test]
fn test_deps_type_checking_dynamic_aliases_do_not_count_as_runtime() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["rich"]
"#,
    )?;
    fs::write(
        root.join("app.py"),
        "from typing import TYPE_CHECKING\nif TYPE_CHECKING:\n    from importlib import import_module as im\nvalue = im('rich.console')\n",
    )?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(output.contains("Unused Dependencies (CSP-R002)"));
    assert!(output.contains("rich"));
    assert!(!output.contains("Missing Dependencies (CSP-R001)"));
    Ok(())
}

#[test]
fn test_deps_json_output() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["unused-dep"]
"#,
    )?;

    fs::write(root.join("main.py"), "import missing_dep\n")?;

    let args = vec![
        "deps".to_owned(),
        root.to_string_lossy().into_owned(),
        "--json".to_owned(),
    ];

    let (code, output) = run_deps_command(args);

    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&output)?;

    assert!(json["unused"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "unused-dep"));
    assert!(json["missing"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "missing_dep"));
    let detail = json["missing_details"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["import_name"] == "missing_dep");
    assert!(
        detail.is_some(),
        "missing_dep should include source evidence"
    );
    let detail = detail.unwrap();
    let location = detail["locations"][0].as_object().unwrap();
    assert!(location["file"].as_str().unwrap().ends_with("main.py"));
    assert_eq!(location["line"], 1);
    assert_eq!(location["column"], 1);

    Ok(())
}

#[test]
fn test_deps_json_output_includes_transitive_and_dev_locations() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["httpx"]

[dependency-groups]
dev = ["pytest"]
"#,
    )?;
    fs::write(
        root.join("uv.lock"),
        r#"
version = 1

[[package]]
name = "httpx"
version = "0.27.0"
dependencies = [{ name = "certifi" }]

[[package]]
name = "certifi"
version = "2024.7.4"
"#,
    )?;
    fs::write(root.join("app.py"), "import certifi\nimport pytest\n")?;

    let (code, output) = run_deps_command(vec![
        "deps".to_owned(),
        root.to_string_lossy().into_owned(),
        "--json".to_owned(),
    ]);

    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&output)?;
    assert_eq!(json["transitive"][0]["import_name"], "certifi");
    assert_eq!(json["transitive"][0]["locations"][0]["line"], 1);
    assert!(json["transitive"][0]["locations"][0]["file"]
        .as_str()
        .unwrap()
        .ends_with("app.py"));
    assert_eq!(json["dev_in_production"][0]["import_name"], "pytest");
    assert_eq!(json["dev_in_production"][0]["locations"][0]["line"], 2);

    Ok(())
}

#[test]
fn test_deps_fail_on_missing_flag() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["requests"]
"#,
    )?;
    fs::write(root.join("main.py"), "import missing_dep\n")?;

    let (code, _output) = run_deps_command(vec![
        "deps".to_owned(),
        root.to_string_lossy().into_owned(),
        "--fail-on-missing".to_owned(),
    ]);

    assert_eq!(code, 1);
    Ok(())
}

#[test]
fn test_deps_fail_on_unused_flag() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["unused-dep"]
"#,
    )?;
    fs::write(root.join("main.py"), "print('hello')\n")?;

    let (code, _output) = run_deps_command(vec![
        "deps".to_owned(),
        root.to_string_lossy().into_owned(),
        "--fail-on-unused".to_owned(),
    ]);

    assert_eq!(code, 1);
    Ok(())
}

#[test]
fn test_deps_fail_on_any_flag() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["requests"]
"#,
    )?;
    fs::write(root.join("main.py"), "import missing_dep\n")?;

    let (code, _output) = run_deps_command(vec![
        "deps".to_owned(),
        root.to_string_lossy().into_owned(),
        "--fail-on-any".to_owned(),
    ]);

    assert_eq!(code, 1);
    Ok(())
}

#[test]
fn test_deps_top_level_fail_on_any_flag() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();
    fs::write(root.join("main.py"), "import missing_dep\n")?;

    let (code, _output) = run_deps_command(vec![
        "--fail-on-any".to_owned(),
        "deps".to_owned(),
        root.to_string_lossy().into_owned(),
    ]);

    assert_eq!(code, 1);
    Ok(())
}
