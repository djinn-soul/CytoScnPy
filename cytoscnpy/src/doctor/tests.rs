use std::fs;
use tempfile::TempDir;

use super::config_detector::detect_configurations;
use super::pyproject::{inspect_pyproject, supplement_configs_with_pyproject};
use super::reliability::evaluate_setup_reliability;
use super::structure::scan_repo_structure;
use super::types::{ConfigCategory, SetupVerdict};

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
fn test_scan_repo_structure() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();

    fs::write(root.join("src/main.py"), "def run():\n    print('hello')\n").unwrap();
    fs::write(root.join("src/lib.py"), "x = 42\n").unwrap();
    fs::write(
        root.join("tests/test_main.py"),
        "def test_run():\n    assert True\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "# Title\n").unwrap();

    let stats = scan_repo_structure(root);

    assert_eq!(stats.total_files, 4);
    assert_eq!(stats.source_files, 3); // main.py, lib.py, README.md
    assert_eq!(stats.test_files, 1); // test_main.py
    assert!(stats.test_to_source_ratio > 0.0);
    assert!(stats.languages.iter().any(|l| l.language == "Python"));
    assert!(stats.languages.iter().any(|l| l.language == "Markdown"));
    assert!(stats.max_directory_depth >= 2);
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
    assert!(!rel.recommendations.is_empty());

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
