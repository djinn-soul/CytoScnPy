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

#[test]
fn test_try_load_reports_invalid_project_configuration() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("pyproject.toml"),
        "[tool.cytoscnpy]\nclones = 'not-a-boolean'\n",
    )
    .unwrap();

    let error = Config::try_load_from_path(dir.path()).unwrap_err();
    let detailed = format!("{error:#}");

    assert!(
        error
            .to_string()
            .contains("invalid [tool.cytoscnpy] configuration"),
        "{error:#}"
    );
    assert!(detailed.contains("expected a boolean"), "{detailed}");
}

#[test]
fn test_try_load_reports_invalid_standalone_configuration() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".cytoscnpy.toml"),
        "[cytoscnpy]\nclones = 'not-a-boolean'\n",
    )
    .unwrap();

    let error = Config::try_load_from_path(dir.path()).unwrap_err();
    let detailed = format!("{error:#}");

    assert!(detailed.contains(".cytoscnpy.toml"), "{detailed}");
    assert!(detailed.contains("expected a boolean"), "{detailed}");
}

#[test]
fn test_legacy_loader_falls_back_to_defaults_on_error() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".cytoscnpy.toml"),
        "[cytoscnpy]\nclones = 'not-a-boolean'\n",
    )
    .unwrap();

    let config = Config::load_from_path(dir.path());

    assert!(config.config_file_path.is_none());
    assert_eq!(config.cytoscnpy.clones, None);
}

#[test]
fn test_cli_reports_invalid_project_configuration() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("pyproject.toml"),
        "[tool.cytoscnpy]\nclones = 'not-a-boolean'\n",
    )
    .unwrap();
    let mut output = Vec::new();

    let error = cytoscnpy::entry_point::run_with_args_to(
        vec![dir.path().to_string_lossy().into_owned()],
        &mut output,
    )
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("invalid [tool.cytoscnpy] configuration"),
        "{error:#}"
    );
}

#[test]
fn test_structured_sanitizer_config() {
    let config: Config = toml::from_str(
        r#"
[cytoscnpy.danger_config.sanitizers.ssrf]
return_value = ["validate_allowed_url"]
guard = ["is_allowed_url"]
side_effect = ["validate_url_or_raise"]

[cytoscnpy.danger_config.sanitizers.sql_injection]
return_value = ["build_parameterized_query"]

[cytoscnpy.danger_config.sanitizers.code_injection]
return_value = ["clean_custom_sink"]
"#,
    )
    .unwrap();

    let sanitizers = &config.cytoscnpy.danger_config.sanitizers;
    assert_eq!(
        sanitizers.ssrf.return_value,
        vec!["validate_allowed_url".to_owned()]
    );
    assert_eq!(sanitizers.ssrf.guard, vec!["is_allowed_url".to_owned()]);
    assert_eq!(
        sanitizers.ssrf.side_effect,
        vec!["validate_url_or_raise".to_owned()]
    );
    assert_eq!(
        sanitizers.sql_injection.return_value,
        vec!["build_parameterized_query".to_owned()]
    );
    assert_eq!(
        sanitizers.code_injection.return_value,
        vec!["clean_custom_sink".to_owned()]
    );
}

#[test]
fn test_legacy_custom_sanitizers_config() {
    let config: Config = toml::from_str(
        r#"
[cytoscnpy.danger_config]
custom_sanitizers = ["legacy_clean"]
"#,
    )
    .unwrap();

    assert_eq!(
        config.cytoscnpy.danger_config.custom_sanitizers,
        Some(vec!["legacy_clean".to_owned()])
    );
}
