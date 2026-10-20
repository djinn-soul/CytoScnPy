use std::fs;
use tempfile::TempDir;

use super::config_detector::detect_configurations;
use super::pyproject::{inspect_pyproject, supplement_configs_with_pyproject};
use super::reliability::evaluate_setup_reliability;
use super::types::{ConfigCategory, SetupVerdict};

#[path = "tests_structure.rs"]
mod structure;

#[test]
fn test_detect_configs() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join(".gitignore"), "target/\n").unwrap();
    fs::write(root.join("Dockerfile"), "FROM python:3.11\n").unwrap();
    fs::write(root.join("pytest.ini"), "[pytest]\n").unwrap();
    fs::write(
        root.join("README.md"),
        "# Project\nRun `pip install -e .`\n",
    )
    .unwrap();

    let configs = detect_configurations(root);

    assert!(configs
        .iter()
        .any(|c| c.category == ConfigCategory::GitIgnore));
    assert!(configs.iter().any(|c| c.category == ConfigCategory::Docker));
    assert!(configs
        .iter()
        .any(|c| c.category == ConfigCategory::TestFramework));
    assert!(configs
        .iter()
        .any(|c| c.category == ConfigCategory::Documentation));
}

#[test]
fn test_pyproject_inspection() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let pyproject_content = r#"
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "demo"
version = "0.1.0"
dependencies = ["fastapi", "uvicorn"]

[project.optional-dependencies]
dev = ["pytest", "ruff", "mypy"]

[tool.ruff]
line-length = 88

[tool.mypy]
strict = true

[tool.pytest.ini_options]
testpaths = ["tests"]
"#;

    fs::write(root.join("pyproject.toml"), pyproject_content).unwrap();

    let inspection = inspect_pyproject(root).expect("Failed to inspect pyproject.toml");
    assert!(inspection.has_ruff);
    assert!(inspection.has_mypy);
    assert!(inspection.has_pytest);
    assert!(!inspection.has_black);
    assert!(inspection.has_flit_or_hatch);
    assert_eq!(inspection.dependencies_count, 2);
    assert_eq!(inspection.dev_dependencies_count, 3);
}

#[test]
fn test_supplement_configs_with_pyproject() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let pyproject_content = "[tool.ruff]\n[tool.mypy]\n";
    let p_path = root.join("pyproject.toml");
    fs::write(&p_path, pyproject_content).unwrap();

    let inspection = inspect_pyproject(root).unwrap();
    let mut configs = Vec::new();

    supplement_configs_with_pyproject(&mut configs, &inspection, &p_path);

    assert!(configs
        .iter()
        .any(|c| c.category == ConfigCategory::Formatter));
    assert!(configs.iter().any(|c| c.category == ConfigCategory::Linter));
    assert!(configs
        .iter()
        .any(|c| c.category == ConfigCategory::TypeChecker));
}

#[test]
fn test_setup_reliability_scoring() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join(".gitignore"), "node_modules/\n").unwrap();
    fs::write(
        root.join("README.md"),
        "# Welcome\nRun `pip install -r requirements.txt` to setup.\n",
    )
    .unwrap();

    let mut configs = detect_configurations(root);
    let rel = evaluate_setup_reliability(root, &configs);

    // Initial minimal setup should have recommendations
    assert!(!rel.has_lockfile);
    assert!(!rel.has_ci);
    assert!(rel.score < 70);
    assert!(
        !rel.recommendations.is_empty(),
        "expected a non-empty value"
    );

    // Add lockfile and test framework
    fs::write(root.join("uv.lock"), "# lockfile").unwrap();
    fs::write(root.join("pytest.ini"), "[pytest]").unwrap();

    configs = detect_configurations(root);
    let updated_rel = evaluate_setup_reliability(root, &configs);

    assert!(updated_rel.has_lockfile);
    assert!(updated_rel.has_tests);
    assert!(updated_rel.score > rel.score);
}

#[test]
fn test_setup_verdict_bands() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let empty_configs = Vec::new();
    let at_risk = evaluate_setup_reliability(root, &empty_configs);
    assert_eq!(at_risk.verdict, SetupVerdict::AtRisk);
}

#[test]
fn test_github_without_workflows_does_not_count_as_ci() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create .github/CODEOWNERS only
    let gh_dir = root.join(".github");
    fs::create_dir_all(&gh_dir).unwrap();
    fs::write(gh_dir.join("CODEOWNERS"), "* @dev\n").unwrap();

    let configs = detect_configurations(root);
    assert!(
        !configs.iter().any(|c| c.category == ConfigCategory::CI),
        ".github directory with only CODEOWNERS must not count as CI"
    );

    let reliability = evaluate_setup_reliability(root, &configs);
    assert!(!reliability.has_ci);

    // Empty .github/workflows directory must also not count
    let workflows_dir = gh_dir.join("workflows");
    fs::create_dir_all(&workflows_dir).unwrap();
    let configs2 = detect_configurations(root);
    assert!(
        !configs2.iter().any(|c| c.category == ConfigCategory::CI),
        "empty .github/workflows directory must not count as CI"
    );

    // Actual workflow file should count as CI
    fs::write(workflows_dir.join("ci.yml"), "name: CI\n").unwrap();
    let configs3 = detect_configurations(root);
    assert!(
        configs3.iter().any(|c| c.category == ConfigCategory::CI),
        ".github/workflows with ci.yml must count as CI"
    );
    let reliability3 = evaluate_setup_reliability(root, &configs3);
    assert!(reliability3.has_ci);
}
