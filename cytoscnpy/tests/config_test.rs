//! Tests for configuration loading and management.
#![allow(clippy::unwrap_used)]

use cytoscnpy::config::Config;
use std::fs;
use std::path::Path;

#[test]
fn test_load_pyproject_toml() {
    let test_dir = Path::new("test_pyproject_config");
    if test_dir.exists() {
        fs::remove_dir_all(test_dir).unwrap();
    }
    fs::create_dir(test_dir).unwrap();

    let pyproject_content = r#"
[tool.cytoscnpy]
confidence = 75
exclude_folders = ["ignore_me"]
"#;
    fs::write(test_dir.join("pyproject.toml"), pyproject_content).unwrap();

    let config = Config::load_from_path(test_dir);

    assert_eq!(config.cytoscnpy.confidence, Some(75));
    assert_eq!(
        config.cytoscnpy.exclude_folders,
        Some(vec!["ignore_me".to_owned()])
    );

    fs::remove_dir_all(test_dir).unwrap();
}

#[test]
fn test_cytoscnpy_toml_precedence() {
    let test_dir = Path::new("test_precedence_config");
    if test_dir.exists() {
        fs::remove_dir_all(test_dir).unwrap();
    }
    fs::create_dir(test_dir).unwrap();

    let pyproject_content = r"
[tool.cytoscnpy]
confidence = 50
";
    fs::write(test_dir.join("pyproject.toml"), pyproject_content).unwrap();

    let cytoscnpy_content = r"
[cytoscnpy]
confidence = 90
";
    fs::write(test_dir.join(".cytoscnpy.toml"), cytoscnpy_content).unwrap();

    let config = Config::load_from_path(test_dir);

    // Should prefer .cytoscnpy.toml (90) over pyproject.toml (50)
    assert_eq!(config.cytoscnpy.confidence, Some(90));

    fs::remove_dir_all(test_dir).unwrap();
}

#[test]
fn test_pyproject_no_cytoscnpy_section() {
    let test_dir = Path::new("test_no_section_config");
    if test_dir.exists() {
        fs::remove_dir_all(test_dir).unwrap();
    }
    fs::create_dir(test_dir).unwrap();

    let pyproject_content = r#"
[tool.other]
foo = "bar"
"#;
    fs::write(test_dir.join("pyproject.toml"), pyproject_content).unwrap();

    let config = Config::load_from_path(test_dir);

    // Should return defaults
    assert_eq!(config.cytoscnpy.confidence, None);
    assert_eq!(config.cytoscnpy.exclude_folders, None);

    fs::remove_dir_all(test_dir).unwrap();
}

#[test]
fn test_full_pyproject_config() {
    let test_dir = Path::new("test_full_config");
    if test_dir.exists() {
        fs::remove_dir_all(test_dir).unwrap();
    }
    fs::create_dir(test_dir).unwrap();

    let pyproject_content = r#"
[tool.cytoscnpy]
confidence = 100
exclude_folders = ["a", "b"]
include_tests = true
secrets = false
danger = true
quality = false
"#;
    fs::write(test_dir.join("pyproject.toml"), pyproject_content).unwrap();

    let config = Config::load_from_path(test_dir);

    assert_eq!(config.cytoscnpy.confidence, Some(100));
    assert_eq!(
        config.cytoscnpy.exclude_folders,
        Some(vec!["a".to_owned(), "b".to_owned()])
    );
    assert_eq!(config.cytoscnpy.include_tests, Some(true));
    assert_eq!(config.cytoscnpy.secrets, Some(false));
    assert_eq!(config.cytoscnpy.danger, Some(true));
    assert_eq!(config.cytoscnpy.quality, Some(false));

    fs::remove_dir_all(test_dir).unwrap();
}

#[test]
fn test_missing_config_files() {
    let test_dir = Path::new("test_missing_config");
    if test_dir.exists() {
        fs::remove_dir_all(test_dir).unwrap();
    }
    fs::create_dir(test_dir).unwrap();

    let config = Config::load_from_path(test_dir);

    // Should return defaults
    assert_eq!(config.cytoscnpy.confidence, None);

    fs::remove_dir_all(test_dir).unwrap();
}
