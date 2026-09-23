//! Integration tests for type checker, CI workflow, and architecture documentation recommendations.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_missing_tooling_and_ci_recommendations() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create minimal Python project with README and tests, but without type checker, CI, or arch docs
    fs::write(
        root.join("pyproject.toml"),
        r#"[project]
name = "test-pkg"
version = "0.1.0"

[tool.ruff]
line-length = 88
"#,
    )
    .unwrap();

    fs::write(
        root.join("README.md"),
        "# Test Project\n\n## Quickstart\n\npip install .\n",
    )
    .unwrap();

    let src_dir = root.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        src_dir.join("main.py"),
        "def compute(x: int) -> int:\n    return x * 2\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "score".to_owned(),
            root.to_string_lossy().into_owned(),
            "--format".to_owned(),
            "llm".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();

    // Verify recommendations appear in the LLM remediation plan
    assert!(
        output_str.contains("Configure static type checking (mypy / pyright)"),
        "Expected type checking recommendation in:\n{output_str}"
    );
    assert!(
        output_str.contains("Configure CI workflow automation (.github/workflows)"),
        "Expected CI recommendation in:\n{output_str}"
    );
    assert!(
        output_str.contains("Document architecture in docs/ARCHITECTURE.md"),
        "Expected architecture doc recommendation in:\n{output_str}"
    );

    // Verify affected files are listed
    assert!(output_str.contains("pyproject.toml"));
    assert!(output_str.contains(".github/workflows/ci.yml"));
    assert!(output_str.contains("docs/ARCHITECTURE.md"));
}

#[test]
fn test_configured_tooling_and_ci_omits_recommendations() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Configure mypy in pyproject.toml
    fs::write(
        root.join("pyproject.toml"),
        r#"[project]
name = "test-pkg"
version = "0.1.0"

[tool.ruff]
line-length = 88

[tool.mypy]
strict = true

[tool.pytest.ini_options]
minversion = "7.0"
"#,
    )
    .unwrap();

    fs::write(
        root.join("README.md"),
        "# Test Project\n\n## Quickstart\n\npip install .\n",
    )
    .unwrap();

    // Create CI workflow
    let workflows = root.join(".github").join("workflows");
    fs::create_dir_all(&workflows).unwrap();
    fs::write(workflows.join("ci.yml"), "name: CI\non: [push]\n").unwrap();

    // Create architecture documentation
    let docs = root.join("docs");
    fs::create_dir_all(&docs).unwrap();
    fs::write(
        docs.join("ARCHITECTURE.md"),
        "# Architecture\n\nSystem design documentation.\n",
    )
    .unwrap();

    // Create source and test
    let src_dir = root.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        src_dir.join("main.py"),
        "def compute(x: int) -> int:\n    return x * 2\n",
    )
    .unwrap();

    let tests_dir = root.join("tests");
    fs::create_dir_all(&tests_dir).unwrap();
    fs::write(
        tests_dir.join("test_main.py"),
        "def test_compute():\n    assert True\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "score".to_owned(),
            root.to_string_lossy().into_owned(),
            "--format".to_owned(),
            "llm".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();

    // With type checking, CI, and architecture docs configured, those recommendations must not appear
    assert!(
        !output_str.contains("Configure static type checking (mypy / pyright)"),
        "Unexpected type checking recommendation when configured"
    );
    assert!(
        !output_str.contains("Configure CI workflow automation (.github/workflows)"),
        "Unexpected CI recommendation when configured"
    );
    assert!(
        !output_str.contains("Document architecture in docs/ARCHITECTURE.md"),
        "Unexpected architecture recommendation when configured"
    );
}
