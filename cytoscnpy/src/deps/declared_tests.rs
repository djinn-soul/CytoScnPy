use super::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_normalize_package_name() {
    assert_eq!(normalize_package_name("Requests"), "requests");
    assert_eq!(normalize_package_name("scikit-learn"), "scikit_learn");
    assert_eq!(normalize_package_name("Flask.restful"), "flask_restful");
}

#[test]
fn test_extract_package_name_from_pep508() {
    // Basic cases
    assert_eq!(
        extract_package_name_from_pep508("requests>=2.28.0").unwrap(),
        "requests"
    );
    assert_eq!(
        extract_package_name_from_pep508("scikit-learn[alldeps]").unwrap(),
        "scikit-learn"
    );
    assert_eq!(
        extract_package_name_from_pep508("numpy ; python_version < '3.9'").unwrap(),
        "numpy"
    );
    // Parenthesized specifier: pkg(>=1.0) — no space before (
    assert_eq!(
        extract_package_name_from_pep508("requests(>=2.28,<3)").unwrap(),
        "requests"
    );
    // URL requirement: pkg @ https://...
    assert_eq!(
        extract_package_name_from_pep508("mylib @ https://example.com/mylib.tar.gz").unwrap(),
        "mylib"
    );
    // VCS requirements — must return None (no PyPI package name)
    assert!(extract_package_name_from_pep508("git+https://github.com/user/repo.git").is_none());
    assert!(extract_package_name_from_pep508("hg+https://bitbucket.org/user/repo").is_none());
    assert!(extract_package_name_from_pep508("svn+https://svn.example.com/repo").is_none());
    // Direct URL — must return None
    assert!(extract_package_name_from_pep508("https://example.com/pkg.tar.gz").is_none());
    // Empty / comment — must return None
    assert!(extract_package_name_from_pep508("").is_none());
    assert!(extract_package_name_from_pep508("# just a comment").is_none());
}

#[test]
fn test_parse_requirements_edge_cases() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let req_path = dir.path().join("requirements.txt");
    fs::write(
        &req_path,
        concat!(
            "requests>=2.28.0\n",
            "numpy; python_version>=\"3.8\"\n", // env marker
            "mylib @ https://example.com/mylib.tar.gz\n", // URL requirement
            "git+https://github.com/user/repo.git\n", // VCS — skip
            "https://example.com/pkg.tar.gz\n", // bare URL — skip
            "-r other-requirements.txt\n",      // flag — skip
            "# a comment\n",                    // comment — skip
            "\n",                               // blank line — skip
        ),
    )?;

    let deps = parse_requirements(&req_path);
    let names: Vec<&str> = deps.iter().map(|d| d.package_name.as_str()).collect();

    assert!(names.contains(&"requests"), "requests missing");
    assert!(names.contains(&"numpy"), "numpy missing");
    assert!(names.contains(&"mylib"), "mylib (@ URL) missing");
    // VCS and bare URLs must be skipped
    assert!(
        !names.iter().any(|n| n.starts_with("git")),
        "VCS should be skipped"
    );
    assert!(
        !names.iter().any(|n| n.starts_with("https")),
        "bare URL should be skipped"
    );
    assert_eq!(deps.len(), 3, "expected exactly 3 deps, got {}", deps.len());
    Ok(())
}

#[test]
fn test_parse_pyproject_basics() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let pyproject_path = dir.path().join("pyproject.toml");
    fs::write(
        &pyproject_path,
        r#"
[project]
dependencies = ["requests", "flask>=2.0"]
[project.optional-dependencies]
test = ["pytest"]
"#,
    )?;

    let deps = parse_pyproject(&pyproject_path);
    let names: Vec<String> = deps.iter().map(|d| d.package_name.clone()).collect();
    assert!(names.contains(&"requests".to_owned()));
    assert!(names.contains(&"flask".to_owned()));
    assert!(names.contains(&"pytest".to_owned()));
    Ok(())
}

#[test]
fn test_parse_pyproject_dependency_groups_tables() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let pyproject_path = dir.path().join("pyproject.toml");
    fs::write(
        &pyproject_path,
        r#"
[project]
dependencies = []

[dependency-groups]
dev = [
  { name = "uvicorn" },
  { include-group = "lint" },
  "pytest>=8.0"
]
"#,
    )?;

    let deps = parse_pyproject(&pyproject_path);
    let names: Vec<String> = deps.iter().map(|d| d.package_name.clone()).collect();
    assert!(names.contains(&"uvicorn".to_owned()));
    assert!(names.contains(&"pytest".to_owned()));
    assert!(!names.contains(&"lint".to_owned()));
    Ok(())
}

#[test]
fn test_parse_requirements_basics() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let req_path = dir.path().join("requirements.txt");
    fs::write(&req_path, "requests==2.28.1\n# comment\nnumpy\n")?;

    let deps = parse_requirements(&req_path);
    let names: Vec<String> = deps.iter().map(|d| d.package_name.clone()).collect();
    assert!(names.contains(&"requests".to_owned()));
    assert!(names.contains(&"numpy".to_owned()));
    Ok(())
}
