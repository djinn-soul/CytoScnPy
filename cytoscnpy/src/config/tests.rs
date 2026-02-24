use super::loader::mark_deprecated_for_test;
use super::*;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_deprecation_detection_toml() {
    let content = r"
[cytoscnpy]
complexity = 10
";
    let mut config = toml::from_str::<Config>(content).unwrap();
    mark_deprecated_for_test(&mut config, content, &["cytoscnpy"]);
    assert!(config.cytoscnpy.uses_deprecated_keys());
    assert_eq!(config.cytoscnpy.max_complexity, Some(10));
}

#[test]
fn test_deprecation_detection_pyproject() {
    let content = r"
[tool.cytoscnpy]
nesting = 5
";
    let pyproject = toml::from_str::<models::PyProject>(content).unwrap();
    let mut config = Config {
        cytoscnpy: pyproject.tool.cytoscnpy,
        config_file_path: None,
    };
    mark_deprecated_for_test(&mut config, content, &["tool", "cytoscnpy"]);
    assert!(config.cytoscnpy.uses_deprecated_keys());
    assert_eq!(config.cytoscnpy.max_nesting, Some(5));
}

#[test]
fn test_load_from_path_no_config() {
    let dir = TempDir::new().unwrap();
    let config = Config::load_from_path(dir.path());
    assert!(config.cytoscnpy.confidence.is_none());
    assert!(config.cytoscnpy.max_complexity.is_none());
}

#[test]
fn test_load_from_path_cytoscnpy_toml() {
    let dir = TempDir::new().unwrap();
    let mut file = std::fs::File::create(dir.path().join(".cytoscnpy.toml")).unwrap();
    writeln!(
        file,
        r"[cytoscnpy]
confidence = 80
max_complexity = 15
"
    )
    .unwrap();

    let config = Config::load_from_path(dir.path());
    assert_eq!(config.cytoscnpy.confidence, Some(80));
    assert_eq!(config.cytoscnpy.max_complexity, Some(15));
}

#[test]
fn test_load_from_path_pyproject_toml() {
    let dir = TempDir::new().unwrap();
    let mut file = std::fs::File::create(dir.path().join("pyproject.toml")).unwrap();
    writeln!(
        file,
        r"[tool.cytoscnpy]
max_lines = 200
max_args = 8
"
    )
    .unwrap();

    let config = Config::load_from_path(dir.path());
    assert_eq!(config.cytoscnpy.max_lines, Some(200));
    assert_eq!(config.cytoscnpy.max_args, Some(8));
}

#[test]
fn test_load_from_path_traverses_up() {
    let dir = TempDir::new().unwrap();
    let nested = dir.path().join("src").join("lib");
    std::fs::create_dir_all(&nested).unwrap();

    let mut file = std::fs::File::create(dir.path().join(".cytoscnpy.toml")).unwrap();
    writeln!(
        file,
        r"[cytoscnpy]
confidence = 90
"
    )
    .unwrap();

    let config = Config::load_from_path(&nested);
    assert_eq!(config.cytoscnpy.confidence, Some(90));
}

#[test]
fn test_load_from_file_path() {
    let dir = TempDir::new().unwrap();
    let mut file = std::fs::File::create(dir.path().join(".cytoscnpy.toml")).unwrap();
    writeln!(
        file,
        r"[cytoscnpy]
min_mi = 65.0
"
    )
    .unwrap();

    let py_file = dir.path().join("test.py");
    std::fs::write(&py_file, "x = 1").unwrap();

    let config = Config::load_from_path(&py_file);
    assert_eq!(config.cytoscnpy.min_mi, Some(65.0));
}
