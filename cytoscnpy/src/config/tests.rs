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

#[test]
fn test_per_file_ignores_multiline_table_and_multiple_whitelist_blocks() {
    let dir = TempDir::new().unwrap();
    let mut file = std::fs::File::create(dir.path().join(".cytoscnpy.toml")).unwrap();
    writeln!(
        file,
        r#"[cytoscnpy]
confidence = 60

[cytoscnpy.per-file-ignores]
"tests/*" = ["CSP-D701"]
"**/__init__.py" = ["CSP-L001"]

[[cytoscnpy.whitelist]]
name = "my_unused_fn"

[[cytoscnpy.whitelist]]
name = "another_fn"
pattern = "wildcard"
"#
    )
    .unwrap();

    let config = Config::load_from_path(dir.path());
    let ignores = config.cytoscnpy.per_file_ignores.unwrap();
    assert_eq!(
        ignores.get("tests/*").unwrap(),
        &vec!["CSP-D701".to_owned()]
    );
    assert_eq!(
        ignores.get("**/__init__.py").unwrap(),
        &vec!["CSP-L001".to_owned()]
    );

    assert_eq!(config.cytoscnpy.whitelist.len(), 2);
    assert_eq!(config.cytoscnpy.whitelist[0].name, "my_unused_fn");
    assert_eq!(config.cytoscnpy.whitelist[1].name, "another_fn");
}

#[test]
fn test_whitelist_compact_array_of_inline_tables() {
    let content = r#"
[cytoscnpy]
whitelist = [
  { name = "fn_one" },
  { name = "fn_two", pattern = "wildcard" },
  { name = "fn_three", file = "src/api/*.py" },
]
"#;
    let config = toml::from_str::<Config>(content).unwrap();
    assert_eq!(config.cytoscnpy.whitelist.len(), 3);
    assert_eq!(config.cytoscnpy.whitelist[0].name, "fn_one");
    assert_eq!(config.cytoscnpy.whitelist[1].name, "fn_two");
    assert_eq!(
        config.cytoscnpy.whitelist[2].file.as_deref(),
        Some("src/api/*.py")
    );
}
