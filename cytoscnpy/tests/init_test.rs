//! Tests for initialization and configuration
use anyhow::Result;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_init_creates_cytoscnpy_toml() -> Result<()> {
    let temp = tempdir()?;
    let mut buffer = Vec::new();

    // Run init in a clean directory
    cytoscnpy::commands::run_init_in(temp.path(), &mut buffer)?;

    let config_path = temp.path().join(".cytoscnpy.toml");
    assert!(config_path.exists());

    let content = fs::read_to_string(&config_path)?;
    assert!(content.contains("[cytoscnpy]"));

    Ok(())
}

#[test]
fn test_init_skips_if_cytoscnpy_toml_exists() -> Result<()> {
    let temp = tempdir()?;
    let config_path = temp.path().join(".cytoscnpy.toml");
    let original_content = "# custom config";
    fs::write(&config_path, original_content)?;

    let mut buffer = Vec::new();
    cytoscnpy::commands::run_init_in(temp.path(), &mut buffer)?;

    // Content should be unchanged
    let content = fs::read_to_string(&config_path)?;
    assert_eq!(content, original_content);

    let output = String::from_utf8(buffer)?;
    assert!(output.contains(".cytoscnpy.toml already exists - skipping"));

    Ok(())
}

#[test]
fn test_init_appends_to_pyproject_toml() -> Result<()> {
    let temp = tempdir()?;
    let pyproject_path = temp.path().join("pyproject.toml");
    fs::write(&pyproject_path, "[project]\nname = \"test\"")?;

    let mut buffer = Vec::new();
    cytoscnpy::commands::run_init_in(temp.path(), &mut buffer)?;

    let content = fs::read_to_string(&pyproject_path)?;
    assert!(content.contains("[tool.cytoscnpy]"));

    let output = String::from_utf8(buffer)?;
    assert!(output.contains("Added default configuration to pyproject.toml"));

    Ok(())
}

#[test]
fn test_init_skips_pyproject_if_already_configured() -> Result<()> {
    let temp = tempdir()?;
    let pyproject_path = temp.path().join("pyproject.toml");
    let original_content = "[tool.cytoscnpy]\nfoo = \"bar\"";
    fs::write(&pyproject_path, original_content)?;

    let mut buffer = Vec::new();
    cytoscnpy::commands::run_init_in(temp.path(), &mut buffer)?;

    let content = fs::read_to_string(&pyproject_path)?;
    assert_eq!(content, original_content);

    let output = String::from_utf8(buffer)?;
    assert!(output.contains("pyproject.toml already contains [tool.cytoscnpy] - skipping"));

    Ok(())
}

#[test]
fn test_init_skips_pyproject_if_cytoscnpy_toml_exists() -> Result<()> {
    let temp = tempdir()?;
    let config_path = temp.path().join(".cytoscnpy.toml");
    fs::write(&config_path, "[cytoscnpy]")?;

    let pyproject_path = temp.path().join("pyproject.toml");
    let pyproject_content = "[project]\nname = \"test\"";
    fs::write(&pyproject_path, pyproject_content)?;

    let mut buffer = Vec::new();
    cytoscnpy::commands::run_init_in(temp.path(), &mut buffer)?;

    // pyproject.toml should be UNTOUCHED
    let content = fs::read_to_string(&pyproject_path)?;
    assert_eq!(content, pyproject_content);

    let output = String::from_utf8(buffer)?;
    assert!(output.contains(".cytoscnpy.toml already exists - skipping"));

    Ok(())
}

/// Uncomments every commented-out TOML setting in an init template, leaving prose comments.
fn uncomment_settings(template: &str) -> String {
    let is_setting = |text: &str| {
        text.starts_with('[')
            || (text.starts_with('"') && text.contains("\" = "))
            || text.starts_with("  {")
            || text.trim_end() == "]"
            || text.split_once(" = ").is_some_and(|(key, _)| {
                !key.is_empty() && key.chars().all(|c| c.is_ascii_lowercase() || c == '_')
            })
    };
    template
        .lines()
        .map(|line| match line.strip_prefix("# ") {
            Some(text) if is_setting(text) => text,
            _ => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn test_init_templates_load_with_every_setting_uncommented() -> Result<()> {
    // Each key below a [table] header belongs to that table, so a template that
    // puts a top-level key after a header breaks as soon as the user enables both.
    for (file, initial) in [
        (".cytoscnpy.toml", None),
        ("pyproject.toml", Some("[project]\nname = \"test\"\n")),
    ] {
        let temp = tempdir()?;
        if let Some(content) = initial {
            fs::write(temp.path().join(file), content)?;
        }
        cytoscnpy::commands::run_init_in(temp.path(), &mut Vec::new())?;

        let path = temp.path().join(file);
        let enabled = uncomment_settings(&fs::read_to_string(&path)?);
        assert!(enabled.contains("max_slop_index = 40"), "{file}: {enabled}");
        fs::write(&path, enabled)?;

        let config = cytoscnpy::config::Config::try_load_from_path(temp.path())
            .map_err(|err| anyhow::anyhow!("{file}: {err:#}"))?;
        assert_eq!(config.cytoscnpy.deslop.max_slop_index, Some(40), "{file}");
        assert_eq!(config.cytoscnpy.whitelist.len(), 3, "{file}");
    }
    Ok(())
}
