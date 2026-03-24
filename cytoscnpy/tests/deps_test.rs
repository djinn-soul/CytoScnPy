//! Integration tests for the `deps` subcommand.
use cytoscnpy::entry_point::run_with_args_to;
use std::fs;
use tempfile::tempdir;

fn run_deps_command(args: Vec<String>) -> (i32, String) {
    let mut buffer = Vec::new();
    let code = run_with_args_to(args, &mut buffer).unwrap_or(1);
    let output = String::from_utf8_lossy(&buffer).into_owned();
    (code, output)
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

    let py_file = r#"
import requests
from sklearn.cluster import KMeans
import missing_dep
import os # stdlib
"#;
    fs::write(root.join("main.py"), py_file)?;

    let args = vec!["deps".to_string(), root.to_string_lossy().to_string()];

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
        "deps".to_string(),
        root.to_string_lossy().to_string(),
        "--ignore-unused".to_string(),
        "unused-dep".to_string(),
        "--ignore-missing".to_string(),
        "missing_dep".to_string(),
    ];

    let (code, output) = run_deps_command(args);

    assert_eq!(code, 0);
    assert!(output.contains("No unused or missing dependencies found!"));

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

    let args = vec!["deps".to_string(), root.to_string_lossy().to_string()];

    let (code, output) = run_deps_command(args);

    assert_eq!(code, 0);
    assert!(output.contains("unused-pkg"));
    assert!(output.contains("requirements.txt"));
    assert!(!output.contains("requests"));

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
        "deps".to_string(),
        root.to_string_lossy().to_string(),
        "--json".to_string(),
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

    Ok(())
}

#[test]
fn test_deps_local_package() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    let pkg_dir = root.join("mypackage");
    fs::create_dir(&pkg_dir)?;
    fs::write(pkg_dir.join("__init__.py"), "")?;

    fs::write(root.join("main.py"), "import mypackage\n")?;

    let args = vec!["deps".to_string(), root.to_string_lossy().to_string()];

    let (code, output) = run_deps_command(args);

    assert_eq!(code, 0);
    assert!(output.contains("No unused or missing dependencies found!"));
    assert!(!output.contains("mypackage"));

    Ok(())
}

#[test]
fn test_deps_mapping_pillow() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["Pillow"]
"#,
    )?;

    fs::write(root.join("main.py"), "import PIL.Image\n")?;

    let args = vec!["deps".to_string(), root.to_string_lossy().to_string()];

    let (code, output) = run_deps_command(args);

    assert_eq!(code, 0);
    assert!(!output.contains("Pillow"));
    assert!(!output.contains("PIL"));

    Ok(())
}
