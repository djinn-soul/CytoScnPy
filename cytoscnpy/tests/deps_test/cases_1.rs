use super::*;

#[test]
fn test_dependency_findings_deserialize_without_location_evidence() -> anyhow::Result<()> {
    let missing: cytoscnpy::deps::MissingDependency = serde_json::from_value(serde_json::json!({
        "import_name": "missing_dep",
    }))?;
    let transitive: cytoscnpy::deps::TransitiveDependency =
        serde_json::from_value(serde_json::json!({
            "import_name": "transitive_dep",
            "package_name": "transitive-dep",
        }))?;
    let dev: cytoscnpy::deps::DevDependencyInProduction =
        serde_json::from_value(serde_json::json!({
            "import_name": "dev_dep",
            "dependency": {
                "package_name": "dev-dep",
                "normalized_name": "dev_dep",
                "is_dev": true,
                "is_optional": false,
                "marker": null,
                "source": "Pyproject",
            },
        }))?;

    assert!(missing.locations.is_empty());
    assert!(transitive.locations.is_empty());
    assert!(dev.locations.is_empty());
    Ok(())
}

#[test]
fn test_deps_unused_and_missing() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    let pyproject = r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = [
    "requests",
    "scikit-learn",
    "unused-dep"
]
"#;
    fs::write(root.join("pyproject.toml"), pyproject)?;

    let py_file = r"
import requests
from sklearn.cluster import KMeans
import missing_dep
import os # stdlib
";
    fs::write(root.join("main.py"), py_file)?;

    let args = vec!["deps".to_owned(), root.to_string_lossy().into_owned()];

    let (code, output) = run_deps_command(args);

    assert_eq!(code, 0);

    assert!(output.contains("Unused Dependencies"));
    assert!(output.contains("unused-dep"));
    assert!(output.contains("Missing Dependencies"));
    assert!(output.contains("missing_dep"));

    assert!(!output.contains("requests"), "requests was reported");
    assert!(!output.contains("scikit-learn"), "scikit-learn reported");
    assert!(!output.contains("os"), "os reported");

    Ok(())
}

#[test]
fn test_deps_ignore_flags() -> anyhow::Result<()> {
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
        "--ignore-unused".to_owned(),
        "unused-dep".to_owned(),
        "--ignore-missing".to_owned(),
        "missing_dep".to_owned(),
    ];

    let (code, output) = run_deps_command(args);

    assert_eq!(code, 0);
    assert!(output.contains("No unused, missing, extra, or orphan dependencies found!"));

    Ok(())
}

#[test]
fn test_deps_requirements_txt() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("requirements.txt"),
        "requests>=2.28.0\nunused-pkg==1.0.0\n",
    )?;
    fs::write(root.join("main.py"), "import requests\n")?;

    let args = vec!["deps".to_owned(), root.to_string_lossy().into_owned()];

    let (code, output) = run_deps_command(args);

    assert_eq!(code, 0);
    assert!(output.contains("unused-pkg"));
    assert!(output.contains("requirements.txt"));
    assert!(!output.contains("requests"));

    Ok(())
}

#[test]
fn test_deps_dev_requirements_not_unused_by_default() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(root.join("requirements-dev.txt"), "pytest==8.0.0\n")?;
    fs::write(root.join("main.py"), "print('hello')\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(
        !output.contains("pytest"),
        "dev dependency should not be reported unused by default"
    );
    assert!(output.contains("No unused, missing, extra, or orphan dependencies found!"));
    Ok(())
}

#[test]
fn test_deps_include_dev_unused_flag() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(root.join("requirements-dev.txt"), "pytest==8.0.0\n")?;
    fs::write(root.join("main.py"), "print('hello')\n")?;

    let (code, output) = run_deps_command(vec![
        "deps".to_owned(),
        root.to_string_lossy().into_owned(),
        "--include-dev-unused".to_owned(),
    ]);

    assert_eq!(code, 0);
    assert!(output.contains("pytest"));
    assert!(output.contains("CSP-R002"));
    Ok(())
}

#[test]
fn test_deps_transitive_dependency_from_uv_lock() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["httpx"]
"#,
    )?;
    fs::write(
        root.join("uv.lock"),
        r#"
version = 1
requires-python = ">=3.10"

[[package]]
name = "httpx"
version = "0.27.0"
dependencies = [{ name = "certifi" }]

[[package]]
name = "certifi"
version = "2024.7.4"
"#,
    )?;
    fs::write(root.join("main.py"), "import certifi\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(output.contains("Transitive Dependencies (CSP-R003)"));
    assert!(output.contains("certifi"));
    assert!(!output.contains("Missing Dependencies (CSP-R001)"));
    Ok(())
}

#[test]
fn test_deps_stdlib_dependency_declared() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["asyncio"]
"#,
    )?;
    fs::write(root.join("main.py"), "import asyncio\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(output.contains("Standard Library Dependencies (CSP-R005)"));
    assert!(output.contains("asyncio"));
    assert!(!output.contains("Unused Dependencies (CSP-R002)"));
    Ok(())
}

#[test]
fn test_deps_dev_dependency_in_production() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = []

[dependency-groups]
dev = ["pytest"]
"#,
    )?;
    fs::write(root.join("app.py"), "import pytest\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(output.contains("Development Dependency Used in Production (CSP-R004)"));
    assert!(output.contains("pytest"));
    Ok(())
}

#[test]
fn test_deps_optional_extra_allowed_in_production() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = []

[project.optional-dependencies]
pytest = ["pytest"]
"#,
    )?;
    fs::write(root.join("plugin.py"), "import pytest\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(!output.contains("CSP-R004"));
    Ok(())
}
