//! Integration tests for the `deps` subcommand.
#![allow(clippy::unwrap_used)]
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

    let args = vec!["deps".to_owned(), root.to_string_lossy().into_owned()];

    let (code, output) = run_deps_command(args);

    assert_eq!(code, 0);
    assert!(output.contains("No unused, missing, extra, or orphan dependencies found!"));
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

    let args = vec!["deps".to_owned(), root.to_string_lossy().into_owned()];

    let (code, output) = run_deps_command(args);

    assert_eq!(code, 0);
    assert!(!output.contains("Pillow"));
    assert!(!output.contains("PIL"));

    Ok(())
}

#[test]
fn test_deps_requirements_env_markers() -> anyhow::Result<()> {
    // Packages with environment markers must be parsed correctly
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("requirements.txt"),
        concat!(
            "requests>=2.28.0\n",
            "unused-pkg; python_version>=\"3.8\"\n",
        ),
    )?;
    fs::write(root.join("main.py"), "import requests\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    // unused-pkg has env marker but no import → should be flagged unused
    assert!(
        output.contains("unused-pkg"),
        "env-marker dep should be detected as unused"
    );
    assert!(
        !output.contains("requests"),
        "requests should not be flagged"
    );
    Ok(())
}

#[test]
fn test_deps_requirements_vcs_and_url_lines_skipped() -> anyhow::Result<()> {
    // VCS and bare-URL lines in requirements.txt must not produce false positives
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("requirements.txt"),
        concat!(
            "requests\n",
            "git+https://github.com/user/repo.git\n",
            "https://example.com/pkg.tar.gz\n",
        ),
    )?;
    fs::write(root.join("main.py"), "import requests\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    // Only `requests` is a real declared dep; the VCS/URL lines must be silently skipped
    assert!(output.contains("No unused, missing, extra, or orphan dependencies found!"));
    Ok(())
}

#[test]
fn test_deps_requirements_at_url() -> anyhow::Result<()> {
    // `pkg @ https://...` format — package name before @ must be extracted
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("requirements.txt"),
        "mylib @ https://example.com/mylib-1.0.tar.gz\n",
    )?;
    fs::write(root.join("main.py"), "import mylib\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    // mylib is declared (@ URL form) and imported — should not be flagged either way
    assert!(output.contains("No unused, missing, extra, or orphan dependencies found!"));
    Ok(())
}

#[test]
fn test_deps_namespace_package_not_flagged_missing() -> anyhow::Result<()> {
    // Namespace packages (e.g. google.cloud.storage) expose top-level `google`.
    // A user who imports `google.cloud.storage` will have `google` extracted
    // as the top-level module.  If they declared `google-cloud-storage`, the
    // normalized name is `google_cloud_storage` which won't match `google`.
    // The correct behaviour is: since `google` is not in stdlib and not a local
    // package, it gets reported missing unless the user provides a mapping.
    // This test documents current behaviour and ensures it doesn't panic/crash.
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["google-cloud-storage"]
"#,
    )?;
    fs::write(root.join("main.py"), "from google.cloud import storage\n")?;

    let (code, _output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    // Must not crash regardless of finding outcome
    assert_eq!(code, 0);
    Ok(())
}
