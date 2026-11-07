use super::*;

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

/// A `TYPE_CHECKING` import is not a runtime import, so it never makes a
/// dependency missing, and a dev dependency used only for typing is not a dev
/// dependency used in production. It is still a *use* of the package though:
/// deleting `pydantic` here would break type checking, so it is not unused.
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
    assert!(!output.contains("Unused Dependencies (CSP-R002)"));
    assert!(!output.contains("Missing Dependencies (CSP-R001)"));
    assert!(!output.contains("Development Dependency Used in Production (CSP-R004)"));
    Ok(())
}
