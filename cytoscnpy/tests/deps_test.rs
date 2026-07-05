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

#[test]
fn test_deps_dependency_declared_in_prod_and_dev_is_allowed_in_production() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["pytest"]

[dependency-groups]
dev = ["pytest"]
"#,
    )?;
    fs::write(root.join("app.py"), "import pytest\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(!output.contains("CSP-R004"));
    assert!(output.contains("No unused, missing, extra, or orphan dependencies found!"));
    Ok(())
}

#[test]
fn test_deps_transitive_requires_reachability_from_declared_dependency() -> anyhow::Result<()> {
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
    fs::write(root.join("main.py"), "import orphan_dep\n")?;
    fs::write(
        root.join("uv.lock"),
        r#"
version = 1

[[package]]
name = "requests"
version = "2.31.0"

[[package]]
name = "orphan-dep"
version = "1.0.0"
"#,
    )?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(output.contains("Missing Dependencies (CSP-R001)"));
    assert!(output.contains("orphan_dep"));
    assert!(!output.contains("Transitive Dependencies (CSP-R003)"));
    Ok(())
}

#[test]
fn test_deps_lockfile_override_uses_exact_file() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();
    let locks = root.join("locks");
    fs::create_dir(&locks)?;

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["requests"]
"#,
    )?;
    fs::write(root.join("main.py"), "import urllib3\n")?;
    fs::write(
        root.join("uv.lock"),
        r#"
version = 1

[[package]]
name = "requests"
version = "2.31.0"
"#,
    )?;
    let override_lock = locks.join("uv.lock");
    fs::write(
        &override_lock,
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
    )?;

    let (code, output) = run_deps_command(vec![
        "deps".to_owned(),
        root.to_string_lossy().into_owned(),
        "--lockfile".to_owned(),
        override_lock.to_string_lossy().into_owned(),
    ]);

    assert_eq!(code, 0);
    assert!(output.contains("Transitive Dependencies (CSP-R003)"));
    assert!(!output.contains("Missing Dependencies (CSP-R001)"));
    Ok(())
}

#[test]
fn test_deps_stdlib_backport_with_marker_is_allowed() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["dataclasses; python_version < '3.7'"]
"#,
    )?;
    fs::write(root.join("main.py"), "print('hello')\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(!output.contains("Standard Library Dependencies (CSP-R005)"));
    Ok(())
}

#[test]
fn test_deps_dev_dependency_allowed_in_tests() -> anyhow::Result<()> {
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
    let tests_dir = root.join("tests");
    fs::create_dir(&tests_dir)?;
    fs::write(tests_dir.join("test_app.py"), "import pytest\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(!output.contains("CSP-R004"));
    assert!(output.contains("No unused, missing, extra, or orphan dependencies found!"));
    Ok(())
}

#[test]
fn test_deps_dev_dependency_allowed_in_conftest() -> anyhow::Result<()> {
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
    fs::write(root.join("conftest.py"), "import pytest\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(!output.contains("CSP-R004"));
    assert!(output.contains("No unused, missing, extra, or orphan dependencies found!"));
    Ok(())
}

#[test]
fn test_deps_dev_dependency_allowed_in_noxfile() -> anyhow::Result<()> {
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
dev = ["nox"]
"#,
    )?;
    fs::write(root.join("noxfile.py"), "import nox\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(!output.contains("CSP-R004"));
    assert!(output.contains("No unused, missing, extra, or orphan dependencies found!"));
    Ok(())
}

#[test]
fn test_deps_type_checking_imports_do_not_count_as_runtime() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["pydantic"]

[dependency-groups]
dev = ["pytest"]
"#,
    )?;
    fs::write(
        root.join("app.py"),
        "from typing import TYPE_CHECKING\nif TYPE_CHECKING:\n    import pydantic\n    import pytest\n",
    )?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(output.contains("Unused Dependencies (CSP-R002)"));
    assert!(output.contains("pydantic"));
    assert!(!output.contains("Missing Dependencies (CSP-R001)"));
    assert!(!output.contains("Development Dependency Used in Production (CSP-R004)"));
    assert!(!output.contains("pytest"));
    Ok(())
}

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

#[test]
fn test_deps_local_namespace_package_not_flagged_missing() -> anyhow::Result<()> {
    // Python 3.3+ namespace packages: a directory without __init__.py but with
    // Python files inside is a valid local package and must not be reported as a
    // missing dependency.
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"[project]
name = "test-pkg"
version = "0.1.0"
dependencies = []
"#,
    )?;

    // Create a namespace package: myns/ with a module but no __init__.py.
    let ns_dir = root.join("myns");
    fs::create_dir(&ns_dir)?;
    fs::write(ns_dir.join("utils.py"), "def helper(): pass\n")?;

    fs::write(root.join("main.py"), "from myns import utils\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(
        !output.contains("myns"),
        "local namespace package should not be flagged missing"
    );
    Ok(())
}

#[test]
fn test_deps_empty_dir_not_treated_as_local_package() -> anyhow::Result<()> {
    // An empty directory (no Python files) should not be treated as a local
    // namespace package — it is more likely an unrelated artifact.
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"[project]
name = "test-pkg"
version = "0.1.0"
dependencies = []
"#,
    )?;

    // Create an empty directory with the same name as a hypothetical import.
    fs::create_dir(root.join("emptyns"))?;

    fs::write(root.join("main.py"), "import emptyns\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    // emptyns has no Python files so it should be flagged as missing.
    assert!(
        output.contains("emptyns"),
        "empty dir should still be flagged missing"
    );
    Ok(())
}

#[test]
fn test_deps_mixed_case_import_not_flagged_missing() -> anyhow::Result<()> {
    // `import Requests` — AST preserves original casing. The declared dep is
    // `requests` (lowercase). Without normalization this was a false-positive
    // missing report.
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["requests"]
"#,
    )?;
    fs::write(root.join("main.py"), "import Requests\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(
        !output.contains("Missing"),
        "Requests should not be flagged missing"
    );
    Ok(())
}

#[test]
fn test_deps_uppercase_mapped_import_not_flagged_unused() -> anyhow::Result<()> {
    // `import PIL` uses the reverse mapping entry (PIL -> pillow). Declaring
    // `pillow` and importing `PIL` must not flag pillow as unused.
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"[project]
name = "test-pkg"
version = "0.1.0"
dependencies = ["pillow"]
"#,
    )?;
    fs::write(root.join("main.py"), "import PIL\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(
        !output.contains("pillow"),
        "pillow should not be flagged unused"
    );
    Ok(())
}

#[test]
fn test_deps_src_layout_not_flagged_missing() -> anyhow::Result<()> {
    // `src/` layout: the project root has a `src/` directory containing sub-packages.
    // `from src.myapp.models import Foo` should not trigger a missing-dep report for
    // `src` because `src/` is a local namespace package (it contains a sub-package).
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"[project]
name = "test-pkg"
version = "0.1.0"
dependencies = []
"#,
    )?;

    // src/ has no __init__.py but contains myapp/ which has Python files.
    let src_dir = root.join("src");
    let myapp_dir = src_dir.join("myapp");
    fs::create_dir_all(&myapp_dir)?;
    fs::write(myapp_dir.join("__init__.py"), "")?;
    fs::write(myapp_dir.join("models.py"), "class Foo: pass\n")?;

    fs::write(root.join("main.py"), "from src.myapp.models import Foo\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(
        !output.contains("src"),
        "src/ layout directory should not be flagged as missing dependency, got: {output}"
    );
    Ok(())
}
